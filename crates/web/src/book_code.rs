//! The book share code (#408): a short code that names a set of active books, so a player can
//! hand the set to another device or a table. Everything here is pure, so native tests cover it.
//!
//! ## What a code means
//!
//! A book has a permanent **slot**: its line number in `sources/published-books.txt`, an
//! append-only list that the build checks. A code does not name book ids and does not depend on
//! the library's order or size. It says, for each slot from 0 up to `described`, whether that
//! book was on. A slot at or above `described` is not described: the sender's library had no
//! such book. A receiver leaves its own choice for such a book as it is, and counts the codes'
//! on-slots that it has no book for as skipped.
//!
//! ## The text form
//!
//! Crockford base32 (`0-9 A-Z` without I, L, O, U), read case-insensitively; `I` and `L` read as
//! `1` and `O` as `0`. Hyphens and spaces are ignored, and the app shows groups of four:
//!
//! ```text
//! version  payload (any length)  check
//! 1        …                     one character
//! ```
//!
//! * **Version** is the first character, `1` for this format. A later format takes another
//!   value and keeps the check, so an old app can say "newer version" instead of "mistyped".
//! * **Check** is the last character: the sum of `(value + 1) * 3^i` over the earlier characters
//!   (`i` counts from 0 at the version), modulo 32. The weights are odd, so any single wrong
//!   character is caught. Two neighbours swapped are caught unless their values differ by
//!   exactly 16. A missing or extra character shifts the weights of the rest, so it is caught
//!   in all but about one case in 32; the `+ 1` makes a `0` character count as well.
//! * **Payload** is a bit stream, packed five bits to a character and padded with zero bits.
//!   The first bit is the mode, picked by the encoder as the shorter of the two:
//!   * `0`, **runs**: Elias-gamma codes for the lengths of alternating runs of off and on
//!     slots, starting with off. The first length is stored plus one (it may be 0); the others
//!     are at least 1. The lengths add up to `described`. A stream that is only padding ends
//!     the list. Because the books of a tier sit next to each other in the first slots (the
//!     file starts in Books order), a preset is a handful of runs.
//!   * `1`, **bits**: Elias-gamma of `described + 1`, then one bit per slot.
//!
//! A code is at most [`MAX_CODE_CHARS`] characters (about 1,240). The decoder stops reading a
//! longer text at that length, before it stores or unpacks anything, so a pasted code or a link
//! cannot make it allocate in proportion to its own size.
//!
//! An Elias-gamma code of `n >= 1` is `bit_length(n) - 1` zero bits, then `n` in binary. It
//! always contains a one, so zero padding can never be read as a number.

use std::collections::BTreeSet;
use std::fmt;

/// Crockford's alphabet.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// The format version this app writes and reads.
pub const VERSION: u8 = 1;

/// The most slots a code may describe. A hostile code cannot ask for more memory than this.
pub const MAX_SLOTS: u32 = cast_iron_schema::MAX_SHARE_SLOTS;

/// The most payload bits a valid code can hold. Bit mode needs about `MAX_SLOTS + 26`; run mode
/// is worst with runs of two (3 gamma bits per 2 slots), about `MAX_SLOTS * 3 / 2 + 2`. The
/// margin covers the mode bit and the rounding.
const MAX_PAYLOAD_BITS: usize = MAX_SLOTS as usize * 3 / 2 + 64;

/// The most characters a valid code can have: the version, the payload and the check. The
/// decoder stops reading at this length, before it stores or unpacks anything, so no input
/// makes it allocate more than this.
pub const MAX_CODE_CHARS: usize = MAX_PAYLOAD_BITS.div_ceil(5) + 2;

/// How many characters the app shows between hyphens.
const GROUP: usize = 4;

/// The weight multiplier of the check. Odd, so every weight is odd.
const WEIGHT_STEP: u32 = 3;

/// What a code says about a set of books.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shared {
    /// The slots from 0 up to this one are described. The sender's library had no book at a
    /// higher slot, so the code says nothing about it.
    pub described: u32,
    /// The slots that were on. Every one is below `described`.
    pub on: BTreeSet<u32>,
}

/// Why a code could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeError {
    /// Nothing but separators.
    Empty,
    /// A character that is not in the alphabet (after the `I`, `L`, `O` rewrites).
    BadCharacter(char),
    /// Fewer characters than a version, a payload character and a check.
    TooShort,
    /// More characters than any code can have ([`MAX_CODE_CHARS`]). It is not read further.
    TooLong,
    /// The check does not match: a typo, a missing character, or an incomplete copy.
    Checksum,
    /// The check matches, but the version is not one this app reads.
    UnsupportedVersion(u8),
    /// The check matches and the version is known, but the payload is not a valid stream.
    Malformed,
}

impl CodeError {
    /// The line that the page shows. It says what to do, and that nothing changed.
    pub fn message(self) -> String {
        match self {
            CodeError::Empty => "Type or paste a code first.".to_string(),
            CodeError::BadCharacter(character) => format!(
                "A code has no “{character}”. Check it for typos. Your books did not change."
            ),
            CodeError::TooShort
            | CodeError::TooLong
            | CodeError::Checksum
            | CodeError::Malformed => {
                "This code does not look right. Check it for typos or a missing part. \
                 Your books did not change."
                    .to_string()
            }
            CodeError::UnsupportedVersion(_) => {
                "This code comes from a newer version of Cast Iron. Update the app, then try \
                 again. Your books did not change."
                    .to_string()
            }
        }
    }
}

impl fmt::Display for CodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

// ---- writing --------------------------------------------------------------------------------

/// The code for `shared`, in groups of four: `1P3K-…`. The shorter of the two payload modes.
pub fn encode(shared: &Shared) -> String {
    group(&encode_plain(shared))
}

/// The code without separators. The check and the tests use it.
fn encode_plain(shared: &Shared) -> String {
    let described = shared.described.min(MAX_SLOTS);
    let on: BTreeSet<u32> = shared.on.range(..described).copied().collect();
    let runs = runs_payload(described, &on);
    let bits = bits_payload(described, &on);
    let payload = if bits.len() < runs.len() { bits } else { runs };
    let mut values = vec![VERSION];
    values.extend(pack(&payload));
    values.push(check(&values));
    values
        .into_iter()
        .map(|value| ALPHABET[usize::from(value)] as char)
        .collect()
}

fn group(plain: &str) -> String {
    let mut text = String::with_capacity(plain.len() + plain.len() / GROUP);
    for (index, character) in plain.chars().enumerate() {
        if index > 0 && index % GROUP == 0 {
            text.push('-');
        }
        text.push(character);
    }
    text
}

/// Mode `0`: the lengths of alternating off and on runs.
fn runs_payload(described: u32, on: &BTreeSet<u32>) -> Vec<bool> {
    let mut lengths: Vec<u32> = Vec::new();
    let mut current = false;
    let mut length = 0u32;
    for slot in 0..described {
        let bit = on.contains(&slot);
        if bit == current {
            length += 1;
        } else {
            lengths.push(length);
            current = bit;
            length = 1;
        }
    }
    lengths.push(length);
    let mut bits = vec![false];
    for (index, length) in lengths.into_iter().enumerate() {
        push_gamma(&mut bits, if index == 0 { length + 1 } else { length });
    }
    bits
}

/// Mode `1`: `described`, then one bit per slot.
fn bits_payload(described: u32, on: &BTreeSet<u32>) -> Vec<bool> {
    let mut bits = vec![true];
    push_gamma(&mut bits, described + 1);
    bits.extend((0..described).map(|slot| on.contains(&slot)));
    bits
}

/// Elias gamma: `bit_length(n) - 1` zeros, then `n`. `n` must be at least 1.
fn push_gamma(bits: &mut Vec<bool>, n: u32) {
    debug_assert!(n >= 1);
    let length = 32 - n.leading_zeros();
    bits.extend(std::iter::repeat_n(false, (length - 1) as usize));
    bits.extend((0..length).rev().map(|shift| (n >> shift) & 1 == 1));
}

/// Packs bits five to a character value, padding the last with zeros.
fn pack(bits: &[bool]) -> Vec<u8> {
    bits.chunks(5)
        .map(|chunk| {
            let mut value = 0u8;
            for index in 0..5 {
                value = (value << 1) | u8::from(chunk.get(index).copied().unwrap_or(false));
            }
            value
        })
        .collect()
}

/// The check value for the characters before it.
fn check(values: &[u8]) -> u8 {
    let mut weight = 1u32;
    let mut sum = 0u32;
    for &value in values {
        sum = (sum + (u32::from(value) + 1) * weight) % 32;
        weight = (weight * WEIGHT_STEP) % 32;
    }
    sum as u8
}

// ---- reading --------------------------------------------------------------------------------

/// Reads a code. It accepts any case, groups, spaces, the Crockford look-alikes, and a pasted
/// share link (`…#/books?set=CODE`).
pub fn decode(text: &str) -> Result<Shared, CodeError> {
    let values = normalize(text)?;
    if values.len() < 3 {
        return Err(CodeError::TooShort);
    }
    let (&last, body) = values.split_last().ok_or(CodeError::TooShort)?;
    if check(body) != last {
        return Err(CodeError::Checksum);
    }
    if body[0] != VERSION {
        return Err(CodeError::UnsupportedVersion(body[0]));
    }
    read_payload(&body[1..])
}

/// The character values of `text`, with separators dropped. It reads at most
/// [`MAX_CODE_CHARS`] of them.
fn normalize(text: &str) -> Result<Vec<u8>, CodeError> {
    // A pasted link carries the code after `set=`; a bare code has no `=`.
    let text = text
        .split_once("set=")
        .map_or(text, |(_, code)| code)
        .split(['&', '#'])
        .next()
        .unwrap_or("");
    let mut values = Vec::new();
    for character in text.chars() {
        if character.is_whitespace() || matches!(character, '-' | '_') {
            continue;
        }
        let upper = character.to_ascii_uppercase();
        let mapped = match upper {
            'I' | 'L' => '1',
            'O' => '0',
            other => other,
        };
        let value = ALPHABET
            .iter()
            .position(|&symbol| symbol as char == mapped)
            .ok_or(CodeError::BadCharacter(upper))?;
        // Stop here, before the vector grows: a code is never longer than this, so a longer
        // text is not one, whatever its check says.
        if values.len() == MAX_CODE_CHARS {
            return Err(CodeError::TooLong);
        }
        values.push(value as u8);
    }
    if values.is_empty() {
        return Err(CodeError::Empty);
    }
    Ok(values)
}

fn unpack(values: &[u8]) -> Vec<bool> {
    values
        .iter()
        .flat_map(|&value| (0..5).rev().map(move |shift| (value >> shift) & 1 == 1))
        .collect()
}

struct Reader<'a> {
    bits: &'a [bool],
    position: usize,
}

impl Reader<'_> {
    /// True when only padding is left: fewer than one character's worth of zero bits.
    fn at_padding(&self) -> bool {
        let rest = &self.bits[self.position..];
        rest.len() < 5 && rest.iter().all(|&bit| !bit)
    }

    fn bit(&mut self) -> Result<bool, CodeError> {
        let bit = *self.bits.get(self.position).ok_or(CodeError::Malformed)?;
        self.position += 1;
        Ok(bit)
    }

    fn gamma(&mut self) -> Result<u32, CodeError> {
        let mut zeros = 0u32;
        while !self.bit()? {
            zeros += 1;
            // Anything this long is past MAX_SLOTS.
            if zeros > 16 {
                return Err(CodeError::Malformed);
            }
        }
        let mut n = 1u32;
        for _ in 0..zeros {
            n = (n << 1) | u32::from(self.bit()?);
        }
        Ok(n)
    }
}

fn read_payload(values: &[u8]) -> Result<Shared, CodeError> {
    let bits = unpack(values);
    let mut reader = Reader {
        bits: &bits,
        position: 0,
    };
    if reader.bit()? {
        read_bits(&mut reader)
    } else {
        read_runs(&mut reader)
    }
}

fn read_runs(reader: &mut Reader<'_>) -> Result<Shared, CodeError> {
    let mut on = BTreeSet::new();
    let mut total = 0u32;
    let mut on_run = false;
    let mut first = true;
    while first || !reader.at_padding() {
        let n = reader.gamma()?;
        let length = if first { n - 1 } else { n };
        first = false;
        let end = total.checked_add(length).ok_or(CodeError::Malformed)?;
        if end > MAX_SLOTS {
            return Err(CodeError::Malformed);
        }
        if on_run {
            on.extend(total..end);
        }
        total = end;
        on_run = !on_run;
    }
    Ok(Shared {
        described: total,
        on,
    })
}

fn read_bits(reader: &mut Reader<'_>) -> Result<Shared, CodeError> {
    let described = reader.gamma()? - 1;
    if described > MAX_SLOTS {
        return Err(CodeError::Malformed);
    }
    let mut on = BTreeSet::new();
    for slot in 0..described {
        if reader.bit()? {
            on.insert(slot);
        }
    }
    if !reader.at_padding() {
        return Err(CodeError::Malformed);
    }
    Ok(Shared { described, on })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shared(described: u32, on: impl IntoIterator<Item = u32>) -> Shared {
        Shared {
            described,
            on: on.into_iter().collect(),
        }
    }

    /// A code with a valid check for these values, whatever the payload says.
    fn sealed(mut values: Vec<u8>) -> String {
        values.push(check(&values));
        values
            .into_iter()
            .map(|value| ALPHABET[usize::from(value)] as char)
            .collect()
    }

    /// A small, repeatable pseudo-random source, so the tests need no crate.
    fn next(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *state >> 33
    }

    #[test]
    fn round_trips_a_spread_of_sets() {
        let mut state = 7u64;
        for described in [0u32, 1, 2, 5, 31, 32, 33, 100, 439, 440, 1000, MAX_SLOTS] {
            for density in [0u64, 1, 5, 50, 95, 100] {
                let on: Vec<u32> = (0..described)
                    .filter(|_| next(&mut state) % 100 < density)
                    .collect();
                let set = shared(described, on);
                let code = encode(&set);
                assert_eq!(decode(&code).as_ref(), Ok(&set), "{described} {density}");
            }
        }
    }

    #[test]
    fn round_trips_the_edges() {
        for set in [
            shared(0, []),
            shared(1, []),
            shared(1, [0]),
            shared(440, []),
            shared(440, 0..440),
            shared(440, [0]),
            shared(440, [439]),
            shared(440, 0..28),
            shared(440, 28..60),
            shared(440, (0..440).filter(|slot| slot % 2 == 0)),
        ] {
            let code = encode(&set);
            assert_eq!(decode(&code).as_ref(), Ok(&set), "{code}");
        }
    }

    #[test]
    fn the_code_starts_with_the_version_and_ends_with_the_check() {
        let code = encode_plain(&shared(440, 0..28));
        assert_eq!(code.chars().next(), Some('1'));
        let values = normalize(&code).unwrap();
        let (&last, body) = values.split_last().unwrap();
        assert_eq!(check(body), last);
    }

    #[test]
    fn groups_of_four_and_a_stable_shape() {
        let code = encode(&shared(440, 0..28));
        for (index, group) in code.split('-').enumerate() {
            assert!(group.len() <= GROUP, "{code}");
            if index > 0 {
                assert!(group.len() == GROUP || code.ends_with(group), "{code}");
            }
        }
        assert!(
            code.chars()
                .all(|c| c == '-' || ALPHABET.contains(&(c as u8)))
        );
    }

    #[test]
    fn a_preset_shaped_set_is_short() {
        // One block of 28 on-slots at the start of 440 slots: the shape of a preset when the
        // file starts in Books order. The value is pinned so a change to the format shows.
        let code = encode_plain(&shared(440, 0..28));
        assert_eq!(code.len(), 8, "{code}");
        // Everything on is shorter still.
        assert!(encode_plain(&shared(440, 0..440)).len() <= 7);
        // Nothing on.
        assert!(encode_plain(&shared(440, [])).len() <= 7);
    }

    #[test]
    fn the_longest_code_is_bounded_by_the_slot_count() {
        // The worst case is the bit mode: about one bit per slot.
        let worst = shared(440, (0..440).filter(|slot| slot % 2 == 0));
        let code = encode_plain(&worst);
        assert!(code.len() <= 440 / 5 + 8, "{}", code.len());
    }

    #[test]
    fn reads_any_case_groups_and_lookalikes() {
        let set = shared(440, 0..28);
        let plain = encode_plain(&set);
        assert_eq!(decode(&plain.to_lowercase()), Ok(set.clone()));
        assert_eq!(decode(&format!("  {}  ", encode(&set))), Ok(set.clone()));
        assert_eq!(decode(&encode(&set).replace('-', " ")), Ok(set.clone()));
        // Crockford: I and L read as 1, O reads as 0.
        let confusable: String = plain
            .chars()
            .map(|c| match c {
                '1' => 'l',
                '0' => 'o',
                other => other,
            })
            .collect();
        assert_eq!(decode(&confusable), Ok(set));
    }

    #[test]
    fn reads_a_pasted_link() {
        let set = shared(440, 0..28);
        let code = encode(&set);
        for text in [
            format!("#/books?set={code}"),
            format!("https://example.test/cast-iron/#/books?set={code}"),
            format!("https://example.test/cast-iron/#/books?set={code}&x=1"),
        ] {
            assert_eq!(decode(&text), Ok(set.clone()), "{text}");
        }
    }

    #[test]
    fn a_wrong_character_is_always_caught() {
        let code = encode_plain(&shared(440, (0..440).filter(|slot| slot % 3 != 0)));
        let values = normalize(&code).unwrap();
        for position in 0..values.len() {
            for other in 0..32u8 {
                if other == values[position] {
                    continue;
                }
                let mut changed = values.clone();
                changed[position] = other;
                let text: String = changed
                    .iter()
                    .map(|&value| ALPHABET[usize::from(value)] as char)
                    .collect();
                assert!(decode(&text).is_err(), "{text}");
            }
        }
    }

    #[test]
    fn swapped_neighbours_are_caught_unless_they_differ_by_sixteen() {
        let code = encode_plain(&shared(440, (0..440).filter(|slot| slot % 3 != 0)));
        let values = normalize(&code).unwrap();
        let (&last, body) = values.split_last().unwrap();
        for position in 0..body.len() - 1 {
            let (a, b) = (body[position], body[position + 1]);
            if a == b {
                continue;
            }
            let mut swapped = body.to_vec();
            swapped.swap(position, position + 1);
            let caught = check(&swapped) != last;
            assert_eq!(caught, a.abs_diff(b) != 16, "{position}: {a} {b}");
        }
    }

    #[test]
    fn a_missing_or_extra_character_is_caught() {
        let code = encode_plain(&shared(440, (0..440).filter(|slot| slot % 3 != 0)));
        for position in 0..code.len() {
            let mut shorter = code.clone();
            shorter.remove(position);
            assert!(decode(&shorter).is_err(), "{shorter}");
            let mut longer = code.clone();
            longer.insert(position, '0');
            assert!(decode(&longer).is_err(), "{longer}");
        }
        assert_eq!(decode(&code[..code.len() - 1]), Err(CodeError::Checksum));
    }

    #[test]
    fn rejects_text_that_is_not_a_code() {
        assert_eq!(decode(""), Err(CodeError::Empty));
        assert_eq!(decode(" - - "), Err(CodeError::Empty));
        assert_eq!(decode("1"), Err(CodeError::TooShort));
        assert_eq!(decode("1Z"), Err(CodeError::TooShort));
        // U is not in Crockford's alphabet.
        assert_eq!(decode("1U00"), Err(CodeError::BadCharacter('U')));
        assert_eq!(decode("hello!"), Err(CodeError::BadCharacter('!')));
    }

    #[test]
    fn the_version_prefix_is_read_before_the_payload() {
        // Version 2 with a correct check: a later format, not a typo.
        let later = sealed(vec![2, 0b10000, 0b00001]);
        assert_eq!(decode(&later), Err(CodeError::UnsupportedVersion(2)));
        // Version 0 is never used.
        assert_eq!(
            decode(&sealed(vec![0, 1, 1])),
            Err(CodeError::UnsupportedVersion(0))
        );
        // The check covers the version: changing only the version character is a checksum
        // error, never a silent read as another version.
        let code = encode_plain(&shared(440, 0..28));
        let other_version = format!("2{}", &code[1..]);
        assert_eq!(decode(&other_version), Err(CodeError::Checksum));
    }

    #[test]
    fn a_valid_check_with_a_broken_payload_is_malformed() {
        // Runs mode (first bit 0) whose first gamma code never ends: all zeros but five bits
        // of them are not padding.
        assert_eq!(decode(&sealed(vec![1, 0, 0])), Err(CodeError::Malformed));
        // A gamma code that is cut off: `0001` promises three more bits.
        assert_eq!(decode(&sealed(vec![1, 0b00001])), Err(CodeError::Malformed));
        // Bits mode that promises more slots than the code holds: gamma(441) then nothing.
        let mut bits = vec![true];
        push_gamma(&mut bits, 441);
        assert_eq!(
            decode(&sealed([vec![1], pack(&bits)].concat())),
            Err(CodeError::Malformed)
        );
        // More slots than the limit.
        let mut bits = vec![false];
        push_gamma(&mut bits, MAX_SLOTS + 2);
        assert_eq!(
            decode(&sealed([vec![1], pack(&bits)].concat())),
            Err(CodeError::Malformed)
        );
    }

    #[test]
    fn bits_beyond_described_are_dropped_when_encoding() {
        let wild = shared(3, [0, 1, 99]);
        assert_eq!(decode(&encode(&wild)), Ok(shared(3, [0, 1])));
    }

    #[test]
    fn every_error_says_that_nothing_changed() {
        for error in [
            CodeError::Empty,
            CodeError::BadCharacter('!'),
            CodeError::TooShort,
            CodeError::TooLong,
            CodeError::Checksum,
            CodeError::UnsupportedVersion(2),
            CodeError::Malformed,
        ] {
            let message = error.message();
            if error != CodeError::Empty {
                assert!(message.contains("did not change"), "{message}");
            }
        }
    }

    /// A code of exactly `length` characters with a correct check, whose payload is zeros.
    fn sealed_of_length(length: usize) -> String {
        sealed([vec![1], vec![0; length - 2]].concat())
    }

    #[test]
    fn the_longest_valid_code_is_under_the_bound() {
        // Run mode is worst with runs of two slots: a zero-length off run, then 2048 runs of 2.
        let mut bits = vec![false];
        push_gamma(&mut bits, 1);
        for _ in 0..2048 {
            push_gamma(&mut bits, 2);
        }
        let code = sealed([vec![1], pack(&bits)].concat());
        assert!(
            code.len() <= MAX_CODE_CHARS,
            "{} > {MAX_CODE_CHARS}",
            code.len()
        );
        let read = decode(&code).unwrap();
        assert_eq!(read.described, MAX_SLOTS);
        assert_eq!(read.on.len(), 2048);
        // Bit mode at the limit of slots.
        let full = shared(MAX_SLOTS, (0..MAX_SLOTS).filter(|slot| slot % 2 == 0));
        let code = encode_plain(&full);
        assert!(code.len() <= MAX_CODE_CHARS);
        assert_eq!(decode(&code), Ok(full));
        // Nothing the encoder writes is longer than the bound.
        assert!(
            encode_plain(&shared(MAX_SLOTS, (0..MAX_SLOTS).filter(|s| s % 3 == 0))).len()
                <= MAX_CODE_CHARS
        );
    }

    #[test]
    fn an_oversized_code_is_refused_whatever_its_check_says() {
        // One character past the bound, with a correct check and with a wrong one.
        let valid = sealed_of_length(MAX_CODE_CHARS + 1);
        assert_eq!(valid.len(), MAX_CODE_CHARS + 1);
        assert_eq!(decode(&valid), Err(CodeError::TooLong));
        let mut invalid = valid.clone();
        let last = invalid.pop().unwrap();
        invalid.push(if last == '0' { '1' } else { '0' });
        assert_eq!(decode(&invalid), Err(CodeError::TooLong));
        // Exactly at the bound it is read as a code, and the check decides.
        let at_bound = sealed_of_length(MAX_CODE_CHARS);
        assert_ne!(decode(&at_bound), Err(CodeError::TooLong));
        assert_eq!(decode(&at_bound), Err(CodeError::Malformed));
        let mut wrong = at_bound.clone();
        let last = wrong.pop().unwrap();
        wrong.push(if last == '0' { '1' } else { '0' });
        assert_eq!(decode(&wrong), Err(CodeError::Checksum));
    }

    #[test]
    fn a_huge_text_is_refused_without_reading_it_all() {
        // Valid characters, a million of them, and with separators between: still too long.
        for text in [
            "7".repeat(1_000_000),
            "7-".repeat(500_000),
            format!("#/books?set={}", "A".repeat(1_000_000)),
            format!("1{}", "0".repeat(100_000)),
        ] {
            assert_eq!(decode(&text), Err(CodeError::TooLong));
        }
        // Separators do not count toward the bound.
        let spaced = sealed_of_length(MAX_CODE_CHARS)
            .chars()
            .flat_map(|c| [c, '-', ' '])
            .collect::<String>();
        assert_ne!(decode(&spaced), Err(CodeError::TooLong));
        // The normalizer itself stops at the bound: it never holds more than one past it.
        assert_eq!(
            normalize(&"7".repeat(MAX_CODE_CHARS + 1)),
            Err(CodeError::TooLong)
        );
        assert_eq!(
            normalize(&"7".repeat(MAX_CODE_CHARS)).map(|v| v.len()),
            Ok(MAX_CODE_CHARS)
        );
        // A bad character before the bound is still named.
        assert_eq!(
            decode(&format!("!{}", "7".repeat(MAX_CODE_CHARS * 2))),
            Err(CodeError::BadCharacter('!'))
        );
    }
}
