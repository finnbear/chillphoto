use http::{HeaderValue, Method, Uri, Version};
use httparse::Status;
use std::io::{self, ErrorKind};
use std::net::TcpStream;
use std::str::FromStr;
use std::time::Instant;
use std::{
    collections::HashMap,
    io::{Read, Write},
    sync::{
        atomic::{AtomicUsize, Ordering},
        LazyLock, Mutex,
    },
    thread::available_parallelism,
    time::Duration,
};

use crate::output::DynLazy;

pub fn serve(
    start: Instant,
    background: bool,
    output: &HashMap<String, (DynLazy<'_, Vec<u8>>, Option<DynLazy<'_, String>>)>,
) {
    let background_threads = &AtomicUsize::new(0);
    let http_threads = &AtomicUsize::new(0);
    let mut queue = output.iter().collect::<Vec<_>>();
    queue.sort_by_key(|(path, _)| !path.contains("_thumbnail"));
    let work = &Mutex::new(queue.iter());
    let available_parallelism = available_parallelism()
        .map(|n| n.get())
        .unwrap_or_default()
        .max(1);
    std::thread::scope(|scope| {
        // Background initialization.
        let background_tasks = if background {
            (available_parallelism / 2).min(4)
        } else {
            0
        };
        for thread in 0..background_tasks {
            let _guard = Guard::new(background_threads);
            scope.spawn(move || {
                let _guard = _guard;
                while let Some((_name, (i, _))) = {
                    let next = work.lock().unwrap().next();
                    next
                } {
                    LazyLock::force(i);
                    //println!("[background] {_name}");
                    while http_threads.load(Ordering::SeqCst) > thread {
                        std::thread::sleep(Duration::from_millis(1000));
                    }
                }

                drop(_guard);

                if background_threads.load(Ordering::SeqCst) == 0 {
                    println!(
                        "({:.1}s) Background processing complete",
                        start.elapsed().as_secs_f32(),
                    );
                }
            });
        }

        let addr = "0.0.0.0:8080";
        let listener = std::net::TcpListener::bind(addr).unwrap();

        println!(
            "({:.1}s) Serving on http://{addr}",
            start.elapsed().as_secs_f32()
        );

        loop {
            let mut stream = if let Ok((stream, _)) = listener.accept() {
                stream
            } else {
                continue;
            };
            scope.spawn(move || loop {
                let mut buf = Vec::new();

                let request = if let Ok(request) = read_request(&mut stream, &mut buf) {
                    request
                } else {
                    return;
                };

                let mut path = request.uri().path().to_owned();
                if path.ends_with('/') {
                    path.push_str("index.html")
                }
                if let Some(query) = request.uri().query() {
                    if query.contains("page=") {
                        use std::fmt::Write;
                        write!(path, "?{query}").unwrap();
                    }
                }

                let response = if let Some((file, hasher)) = output.get(&path) {
                    while http_threads.load(Ordering::SeqCst) >= available_parallelism {
                        std::thread::sleep(Duration::from_secs(50));
                    }

                    let _guard = Guard::new(http_threads);

                    let mut builder = http::Response::builder()
                        .version(request.version())
                        .status(200);

                    if let Some(hasher) = hasher {
                        builder = builder.header("Etag", (&***hasher).to_owned());
                    }
                    if path.ends_with(".svg") {
                        // Help Chrome
                        builder = builder.header("Content-Type", "image/svg+xml\r\n");
                    }
                    builder.body((&***file).to_vec()).unwrap()
                } else {
                    http::Response::builder()
                        .version(request.version())
                        .status(404)
                        .body(b"not found".to_vec())
                        .unwrap()
                };

                println!("[{}] {}", response.status(), request.uri());

                if write_response(&mut stream, response).is_err() {
                    return;
                }
            });
        }
    });
}

struct Guard<'a>(&'a AtomicUsize);

impl<'a> Guard<'a> {
    pub fn new(counter: &'a AtomicUsize) -> Self {
        counter.fetch_add(1, Ordering::SeqCst);
        Self(counter)
    }
}

impl<'a> Drop for Guard<'a> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

fn read_request(stream: &mut TcpStream, buf: &mut Vec<u8>) -> io::Result<http::Request<Vec<u8>>> {
    loop {
        let mut tmp = [0u8; 1024];
        match stream.read(&mut tmp)? {
            0 => {
                return Err(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "read 0 bytes from remote",
                ))
            }
            n => {
                buf.extend_from_slice(&tmp[0..n]);
            }
        };

        let mut headers = [httparse::EMPTY_HEADER; 128];
        let mut parse_req = httparse::Request::new(&mut headers);
        let res = parse_req.parse(&buf).unwrap();
        if let Status::Complete(body) = res {
            let method =
                if let Some(method) = parse_req.method.and_then(|m| Method::from_str(m).ok()) {
                    method
                } else {
                    return Err(io::Error::new(ErrorKind::InvalidData, "invalid method"));
                };
            let uri = if let Some(uri) = parse_req.path.and_then(|p| Uri::from_str(p).ok()) {
                uri
            } else {
                return Err(io::Error::new(ErrorKind::InvalidData, "invalid URI"));
            };
            let mut builder = http::Request::builder().method(method).uri(uri).version(
                if parse_req.version == Some(1) {
                    Version::HTTP_11
                } else {
                    Version::HTTP_10
                },
            );
            for header in parse_req.headers {
                builder = builder.header(header.name, header.value);
            }
            let content_length =
                if let Some(n) = builder.headers_ref().and_then(|h| h.get("content-length")) {
                    if let Some(n) = n.to_str().ok().and_then(|n| n.parse::<usize>().ok()) {
                        n
                    } else {
                        return Err(io::Error::new(
                            ErrorKind::InvalidData,
                            "invalid content-length",
                        ));
                    }
                } else {
                    0
                };

            if content_length > 1024 * 1024 {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "excessive content-length",
                ));
            }

            // Consume the request.
            buf.splice(0..body, std::iter::empty());

            // Allocate for reading body.
            buf.resize(buf.len().max(content_length), 0);

            stream.read_exact(&mut buf[0..content_length])?;

            let new_buf = buf.split_off(content_length);
            let body = std::mem::replace(buf, new_buf);
            let request = builder.body(body).unwrap();

            break Ok(request);
        }
    }
}

fn write_response(stream: &mut TcpStream, mut response: http::Response<Vec<u8>>) -> io::Result<()> {
    let mut header = format!(
        "{:?} {} {}\r\n",
        response.version(),
        response.status().as_u16(),
        response.status().canonical_reason().unwrap()
    );

    let content_length = response.body().len();
    response.headers_mut().insert(
        "Content-Length",
        HeaderValue::from_str(&content_length.to_string()).unwrap(),
    );

    for (name, value) in response.headers() {
        use std::fmt::Write;
        write!(header, "{}: {}\r\n", name, value.to_str().unwrap_or("")).unwrap();
    }

    header.push_str("\r\n");
    stream.write_all(header.as_bytes())?;
    stream.write_all(&response.body())?;
    stream.flush()?;
    Ok(())
}
