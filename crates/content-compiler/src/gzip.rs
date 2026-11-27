//! Compresses the library files (docs/decisions/2026-11-27-precompressed-library.md, section 2).
//!
//! Every file except `manifest.json` is stored as one gzip member. The bytes must depend only on
//! the input (AGENTS.md rule 5): level 6, one member, and a header with mtime 0, no file name, no
//! comment, no extra field, and operating system 255 (unknown). `Cargo.lock` pins `flate2` and
//! `miniz_oxide`, so a change to either can change the bytes and so the content version; the
//! golden test below fails when that happens, and the lock update then says so in its pull request.

use flate2::{Compression, GzBuilder, write::GzEncoder};
use std::collections::BTreeMap;
use std::io::Write;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The compression level of the library: 0.9% larger than level 9 and up to twice as fast.
const LEVEL: u32 = 6;

/// The header's operating-system byte: unknown, so a build on any host gives the same bytes.
const OS_UNKNOWN: u8 = 255;

/// The suffix of a stored library file.
pub const SUFFIX: &str = ".gz";

/// Compresses `bytes` into one gzip member with the fixed header.
pub fn gzip(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder: GzEncoder<Vec<u8>> = GzBuilder::new()
        .mtime(0)
        .operating_system(OS_UNKNOWN)
        .write(Vec::new(), Compression::new(LEVEL));
    encoder
        .write_all(bytes)
        .map_err(|e| format!("cannot compress: {e}"))?;
    encoder
        .finish()
        .map_err(|e| format!("cannot compress: {e}"))
}

/// Compresses every file on worker threads and returns the result by its stored name: the name of
/// the file plus [`SUFFIX`].
///
/// The output of each file depends only on that file, so the result does not depend on how the
/// workers share the work. The largest files go first, so the largest one (`search.bin`, about a
/// third of the bytes) bounds the wall time.
pub fn gzip_all(files: &BTreeMap<String, Vec<u8>>) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut order: Vec<(&String, &Vec<u8>)> = files.iter().collect();
    order.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
    let next = AtomicUsize::new(0);
    let done: Mutex<BTreeMap<String, Vec<u8>>> = Mutex::new(BTreeMap::new());
    let failed: Mutex<Option<String>> = Mutex::new(None);
    let workers = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(order.len().max(1));
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let at = next.fetch_add(1, Ordering::Relaxed);
                    let Some((name, bytes)) = order.get(at) else {
                        return;
                    };
                    match gzip(bytes) {
                        Ok(stored) => {
                            done.lock()
                                .expect("a worker panicked")
                                .insert(format!("{name}{SUFFIX}"), stored);
                        }
                        Err(e) => {
                            *failed.lock().expect("a worker panicked") =
                                Some(format!("{name}: {e}"));
                            return;
                        }
                    }
                }
            });
        }
    });
    if let Some(e) = failed.into_inner().expect("a worker panicked") {
        return Err(e);
    }
    Ok(done.into_inner().expect("a worker panicked"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::io::Read;

    /// A fixed input that is long enough to need real deflate blocks and repeats enough to compress.
    fn fixed_input() -> Vec<u8> {
        (0..20_000u32)
            .flat_map(|i| {
                format!("{{\"id\":\"ref/{}\",\"n\":{}}}\n", i % 977, i * 31 % 1013).into_bytes()
            })
            .collect()
    }

    #[test]
    fn the_header_is_fixed() {
        let stored = gzip(b"{}").unwrap();
        assert_eq!(
            stored[..10],
            [0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff]
        );
    }

    #[test]
    fn the_output_decodes_to_the_input() {
        let input = fixed_input();
        let stored = gzip(&input).unwrap();
        assert!(stored.len() < input.len() / 4);
        let mut decoded = Vec::new();
        flate2::read::GzDecoder::new(stored.as_slice())
            .read_to_end(&mut decoded)
            .unwrap();
        assert_eq!(decoded, input);
    }

    /// The golden hash: a change of `flate2` or `miniz_oxide` that changes the bytes fails here,
    /// and so changes the content version of every library. Update the value in the pull request
    /// that updates `Cargo.lock`, and say so there.
    const GOLDEN_SHA256: &str = "08ed627f7535babb46624608f46619d85f46c0736614e8512cc333c25672aacc";

    #[test]
    fn the_output_for_a_fixed_input_is_the_committed_bytes() {
        let hash = format!("{:x}", Sha256::digest(gzip(&fixed_input()).unwrap()));
        assert_eq!(hash, GOLDEN_SHA256, "the compressor's output changed");
    }

    #[test]
    fn compressing_twice_gives_the_same_bytes() {
        assert_eq!(gzip(&fixed_input()).unwrap(), gzip(&fixed_input()).unwrap());
    }

    #[test]
    fn threaded_compression_equals_one_file_at_a_time() {
        let files: BTreeMap<String, Vec<u8>> = (0..9)
            .map(|i| {
                (
                    format!("f{i}.json"),
                    fixed_input()[..(i + 1) * 5_000].to_vec(),
                )
            })
            .collect();
        let threaded = gzip_all(&files).unwrap();
        assert_eq!(threaded.len(), files.len());
        for (name, bytes) in &files {
            assert_eq!(
                threaded[&format!("{name}.gz")],
                gzip(bytes).unwrap(),
                "{name}"
            );
        }
    }

    #[test]
    fn an_empty_set_of_files_compresses_to_an_empty_set() {
        assert!(gzip_all(&BTreeMap::new()).unwrap().is_empty());
    }
}
