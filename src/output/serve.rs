use http::{Method, Uri, Version};
use httparse::Status;
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

pub fn serve(
    start: Instant,
    output: &HashMap<String, LazyLock<Vec<u8>, Box<dyn FnOnce() -> Vec<u8> + Send + Sync + '_>>>,
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
        let cpus = (available_parallelism / 2).min(4);
        for thread in 0..cpus {
            let _guard = Guard::new(background_threads);
            scope.spawn(move || {
                let _guard = _guard;
                while let Some((_, i)) = {
                    let next = work.lock().unwrap().next();
                    next
                } {
                    LazyLock::force(i);
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
            scope.spawn(move || {
                let mut buf = Vec::new();

                let (request, _body) = loop {
                    let mut tmp = [0u8; 1024];
                    match stream.read(&mut tmp) {
                        Ok(0) => return,
                        Ok(n) => {
                            buf.extend_from_slice(&tmp[0..n]);
                        }
                        Err(_) => {
                            return;
                        }
                    };

                    let mut headers = [httparse::EMPTY_HEADER; 128];
                    let mut parse_req = httparse::Request::new(&mut headers);
                    let res = parse_req.parse(&buf).unwrap();
                    if let Status::Complete(body) = res {
                        let method = if let Some(method) =
                            parse_req.method.and_then(|m| Method::from_str(m).ok())
                        {
                            method
                        } else {
                            return;
                        };
                        let uri =
                            if let Some(uri) = parse_req.path.and_then(|p| Uri::from_str(p).ok()) {
                                uri
                            } else {
                                return;
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
                        let request = builder.body(Vec::<u8>::new()).unwrap();

                        break (request, body);
                    }
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

                let response = if let Some(file) = output.get(&path) {
                    while http_threads.load(Ordering::SeqCst) >= available_parallelism {
                        std::thread::sleep(Duration::from_secs(50));
                    }

                    let _guard = Guard::new(http_threads);

                    http::Response::builder()
                        .version(request.version())
                        .status(200)
                        .body((&***file).to_vec())
                        .unwrap()
                } else {
                    http::Response::builder()
                        .version(request.version())
                        .status(404)
                        .body(b"not found".to_vec())
                        .unwrap()
                };

                println!("[{}] {}", response.status(), request.uri());

                let status_line = format!(
                    "{:?} {} {}\r\n",
                    response.version(),
                    response.status().as_u16(),
                    response.status().canonical_reason().unwrap()
                );

                let mut headers = String::new();
                for (name, value) in response.headers() {
                    headers.push_str(&format!("{}: {}\r\n", name, value.to_str().unwrap_or("")));
                }

                let body: &[u8] = response.body().as_ref();
                let content_length = body.len();
                headers.push_str(&format!("Content-Length: {}\r\n", content_length));
                if path.ends_with(".svg") {
                    // Help Chrome
                    headers.push_str("Content-Type: image/svg+xml\r\n");
                }
                headers.push_str("\r\n");
                if stream.write_all(status_line.as_bytes()).is_err() {
                    return;
                }
                if stream.write_all(headers.as_bytes()).is_err() {
                    return;
                }
                if stream.write_all(body).is_err() {
                    return;
                }
                let _ = stream.flush();
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
