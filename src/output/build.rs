use crate::{gallery::Gallery, output::DynLazy, util::recursively_remove_empty_dirs_of_contents};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    collections::HashMap,
    fs,
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};
use wax::Glob;

pub fn build(
    start: Instant,
    gallery: &Gallery,
    output: &HashMap<String, (DynLazy<'_, Vec<u8>>, Option<DynLazy<'_, String>>)>,
) {
    let progress = ProgressBar::new(output.len() as u64)
        .with_message("Saving website...")
        .with_style(
            ProgressStyle::default_bar()
                .template("{msg} {wide_bar} {pos}/{len} {eta}")
                .unwrap(),
        )
        .with_elapsed(start.elapsed());

    let reused = AtomicUsize::new(0);
    let total = AtomicUsize::new(0);
    let mut removals = 0usize;

    // Remove obsolete files.
    for file in Glob::new("**").unwrap().walk(&gallery.config.output) {
        let file = file.unwrap();
        if !file.file_type().is_file() {
            continue;
        }
        let path = format!("/{}", file.matched().complete());
        if output.get(&path).is_none() {
            //println!("removing obsolete {path}");
            fs::remove_file(file.path()).unwrap();
            removals += 1;
        }
    }
    recursively_remove_empty_dirs_of_contents(&gallery.config.output).unwrap();

    output.par_iter().for_each(|(path, (generator, hasher))| {
        let path = gallery.config.subdirectory(path.strip_prefix('/').unwrap());

        let new_hash = hasher.as_ref().map(|hasher| (&**hasher).as_bytes());

        let mut reuse = false;
        if let Some(new_hash) = new_hash {
            if fs::exists(&path).unwrap() {
                if let Ok(Some(old_hash)) = fsquirrel::get(&path, "chillphotohash") {
                    if old_hash == new_hash {
                        reuse = true;
                    }
                }
            }

            total.fetch_add(1, Ordering::Relaxed);
            if reuse {
                reused.fetch_add(1, Ordering::Relaxed);
            }
        }

        if !reuse {
            let contents = &**generator;
            if let Some((dir, _)) = path.rsplit_once('/') {
                std::fs::create_dir_all(dir).unwrap();
            }
            let _ = fsquirrel::remove(&path, "chillphotohash");
            std::fs::write(&path, contents).unwrap();
            if let Some(new_hash) = new_hash {
                fsquirrel::set(&path, "chillphotohash", new_hash).unwrap();
            }
        }

        progress.inc(1);
    });

    progress.finish_and_clear();

    println!(
        "({:.1}s) Saved website to {}, reusing {}/{} images files, removed {removals} obsolete files",
        start.elapsed().as_secs_f32(),
        gallery.config.output,
        reused.load(Ordering::Relaxed),
        total.load(Ordering::Relaxed),
    );
}
