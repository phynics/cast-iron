//! A gzip reader for a browser without `DecompressionStream`. It reads one RFC 1952 member and
//! inflates it with `miniz_oxide`.
//!
//! The compiler writes a fixed 10-byte header with no optional field, but this reader parses
//! all of them (FEXTRA, FNAME, FCOMMENT, FHCRC), so a file that another tool gzipped still
//! reads. It checks the trailer's size. It does not compute the CRC-32: the caller compares the
//! SHA-256 of the result with the manifest, which covers every bit the CRC covers.

use miniz_oxide::inflate::TINFLStatus;
use miniz_oxide::inflate::core::inflate_flags::{
    TINFL_FLAG_IGNORE_ADLER32, TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
};
use miniz_oxide::inflate::core::{DecompressorOxide, decompress};

const ID1: u8 = 0x1f;
const ID2: u8 = 0x8b;
const DEFLATE: u8 = 8;
const FHCRC: u8 = 1 << 1;
const FEXTRA: u8 = 1 << 2;
const FNAME: u8 = 1 << 3;
const FCOMMENT: u8 = 1 << 4;
/// Bits 5 to 7 are reserved and must be zero.
const RESERVED: u8 = 0xe0;
const HEADER: usize = 10;
const TRAILER: usize = 8;

/// Where the deflate stream starts in a gzip member, and the bytes that it ends before.
fn deflate_range(member: &[u8]) -> Result<(usize, usize), &'static str> {
    const NOT_GZIP: &str = "not a gzip file";
    const CUT: &str = "the gzip header is cut off";
    if member.len() < HEADER + TRAILER || member[..3] != [ID1, ID2, DEFLATE] {
        return Err(NOT_GZIP);
    }
    let flags = member[3];
    if flags & RESERVED != 0 {
        return Err(NOT_GZIP);
    }
    let end = member.len() - TRAILER;
    let mut at = HEADER;
    if flags & FEXTRA != 0 {
        let length = member.get(at..at + 2).ok_or(CUT)?;
        at += 2 + usize::from(u16::from_le_bytes([length[0], length[1]]));
    }
    for flag in [FNAME, FCOMMENT] {
        if flags & flag != 0 {
            let rest = member.get(at..end).ok_or(CUT)?;
            at += rest.iter().position(|&b| b == 0).ok_or(CUT)? + 1;
        }
    }
    if flags & FHCRC != 0 {
        at += 2;
    }
    if at > end {
        return Err(CUT);
    }
    Ok((at, end))
}

/// Inflates one gzip member into exactly `size` bytes, the decoded size that the manifest
/// lists. The output buffer has that size from the start, so a bomb cannot grow the WASM memory
/// (it never shrinks) and a large file does not pay for a doubling vector. The member must
/// inflate to exactly `size` bytes, the trailer must say the same, and nothing may follow the
/// member (one member only).
pub fn gunzip(member: &[u8], size: usize) -> Result<Vec<u8>, &'static str> {
    let (start, end) = deflate_range(member)?;
    let mut decoded = vec![0; size];
    // One call is the whole member: the input is complete and the output has its full size.
    let (status, consumed, written) = decompress(
        &mut DecompressorOxide::new(),
        &member[start..end],
        &mut decoded,
        0,
        TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF | TINFL_FLAG_IGNORE_ADLER32,
    );
    // The stream must end exactly where the trailer starts: bytes after it (a second member,
    // or junk) are refused, because `DecompressionStream` and this reader would disagree on them.
    if status != TINFLStatus::Done || consumed != end - start {
        return Err("cannot inflate the gzip data");
    }
    // The trailer is the CRC-32 (not read), then ISIZE: the size modulo 2^32.
    let isize = u32::from_le_bytes([
        member[end + 4],
        member[end + 5],
        member[end + 6],
        member[end + 7],
    ]);
    if written != size || !u32::try_from(size).is_ok_and(|s| s == isize) {
        return Err("the gzip data is not the listed size");
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One stored (uncompressed) deflate block, final.
    fn stored(data: &[u8]) -> Vec<u8> {
        let len = data.len() as u16;
        let mut block = vec![0x01];
        block.extend_from_slice(&len.to_le_bytes());
        block.extend_from_slice(&(!len).to_le_bytes());
        block.extend_from_slice(data);
        block
    }

    fn member(flags: u8, optional: &[u8], data: &[u8]) -> Vec<u8> {
        let mut out = vec![ID1, ID2, DEFLATE, flags, 0, 0, 0, 0, 0, 255];
        out.extend_from_slice(optional);
        out.extend_from_slice(&stored(data));
        // The CRC-32 is not read; any value stands in.
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out
    }

    /// `printf 'hello hello hello\n' | gzip -n -6`: a real compressed block, so the Huffman
    /// path runs and not only the stored one.
    const HELLO_GZ: [u8; 29] = [
        0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0xcb, 0x48, 0xcd, 0xc9, 0xc9,
        0x57, 0xc8, 0x40, 0x90, 0x5c, 0x00, 0x3b, 0x7c, 0x8a, 0xdf, 0x12, 0x00, 0x00, 0x00,
    ];

    #[test]
    fn reads_a_compressed_member() {
        assert_eq!(gunzip(&HELLO_GZ, 18).unwrap(), b"hello hello hello\n");
    }

    #[test]
    fn reads_the_compiler_header_and_an_empty_file() {
        assert_eq!(gunzip(&member(0, &[], b""), 0).unwrap(), b"");
        assert_eq!(gunzip(&member(0, &[], b"abc"), 3).unwrap(), b"abc");
    }

    #[test]
    fn skips_every_optional_header_field() {
        let mut optional = Vec::new();
        optional.extend_from_slice(&3u16.to_le_bytes()); // FEXTRA, 3 bytes
        optional.extend_from_slice(&[9, 9, 9]);
        optional.extend_from_slice(b"name.json\0"); // FNAME
        optional.extend_from_slice(b"a comment\0"); // FCOMMENT
        optional.extend_from_slice(&[0xaa, 0xbb]); // FHCRC
        let flags = FEXTRA | FNAME | FCOMMENT | FHCRC;
        assert_eq!(
            gunzip(&member(flags, &optional, b"payload"), 7).unwrap(),
            b"payload"
        );
        // Each field alone.
        assert_eq!(
            gunzip(&member(FNAME, b"x\0", b"payload"), 7).unwrap(),
            b"payload"
        );
        assert_eq!(
            gunzip(&member(FEXTRA, &[0, 0], b"payload"), 7).unwrap(),
            b"payload"
        );
    }

    #[test]
    fn rejects_a_bad_header() {
        let good = member(0, &[], b"abc");
        let mut magic = good.clone();
        magic[0] = 0;
        assert!(gunzip(&magic, 8).is_err());
        let mut method = good.clone();
        method[2] = 7;
        assert!(gunzip(&method, 8).is_err());
        let mut reserved = good.clone();
        reserved[3] = 0x20;
        assert!(gunzip(&reserved, 8).is_err());
        assert!(gunzip(&good[..12], 8).is_err());
        // A name that never ends, and an extra field longer than the file.
        assert!(gunzip(&member(FNAME, b"", b"abc"), 8).is_err());
        let mut long_extra = member(FEXTRA, &[0xff, 0xff], b"abc");
        long_extra.truncate(20);
        assert!(gunzip(&long_extra, 8).is_err());
    }

    #[test]
    fn refuses_a_second_member_and_trailing_bytes() {
        let one = member(0, &[], b"abc");
        // Two members: `DecompressionStream` would decode both, so this reader refuses them.
        let two = [one.clone(), one.clone()].concat();
        assert!(gunzip(&two, 3).is_err());
        assert!(gunzip(&two, 6).is_err());
        // Junk after the trailer.
        let junk = [one.clone(), vec![0u8; 5]].concat();
        assert!(gunzip(&junk, 3).is_err());
        assert_eq!(gunzip(&one, 3).unwrap(), b"abc");
    }

    #[test]
    fn rejects_a_wrong_trailer_size_a_cut_stream_and_a_bomb() {
        let mut size = member(0, &[], b"abc");
        let last = size.len() - 4;
        size[last] = 4;
        assert!(gunzip(&size, 3).is_err());
        let cut = &HELLO_GZ[..HELLO_GZ.len() - 12];
        assert!(gunzip(cut, 18).is_err());
        // A buffer that is too small (a bomb) and one that is too large.
        assert!(gunzip(&HELLO_GZ, 5).is_err());
        assert!(gunzip(&HELLO_GZ, 40).is_err());
    }
}
