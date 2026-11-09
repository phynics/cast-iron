//! On-disk layout: header, then three independently decodable sections
//! (metadata, postings, stored text), each postcard-encoded.

use serde::{Deserialize, Serialize};

pub(crate) const MAGIC: &[u8; 4] = b"CIDX";
pub(crate) const HEADER_LEN: usize = 4 + 4 + 3 * 4;

/// Errors from decoding an index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexError {
    BadMagic,
    UnsupportedVersion(u32),
    Truncated,
    Corrupt(&'static str),
}

impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadMagic => write!(f, "not a search index (bad magic)"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported index format version {v}"),
            Self::Truncated => write!(f, "index data is truncated"),
            Self::Corrupt(m) => write!(f, "corrupt index: {m}"),
        }
    }
}

impl std::error::Error for IndexError {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct DocMeta {
    pub id: String,
    pub title: String,
    pub aliases: Vec<String>,
    pub kind: String,
    pub source: String,
    pub section: String,
    pub facets: Vec<(String, String)>,
    /// The chapter page of a book-section document, and its row anchor. Empty for entries.
    pub chapter: String,
    pub anchor: String,
    pub len_title: u32,
    pub len_alias: u32,
    pub len_text: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct MetaSection {
    pub docs: Vec<DocMeta>,
    pub avg_title: f32,
    pub avg_alias: f32,
    pub avg_text: f32,
}

/// Sorted term table plus delta/varint-coded postings.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub(crate) struct PostingsSection {
    pub terms: Vec<String>,
    pub dfs: Vec<u32>,
    /// `terms.len() + 1` offsets into `blob`.
    pub offsets: Vec<u32>,
    pub blob: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub(crate) struct TextSection {
    pub texts: Vec<String>,
}

/// One decoded posting: doc ordinal and per-field term frequencies.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Posting {
    pub doc: u32,
    pub tf_title: u32,
    pub tf_alias: u32,
    pub tf_text: u32,
}

pub(crate) fn put_varint(out: &mut Vec<u8>, mut v: u32) {
    while v >= 0x80 {
        out.push((v as u8 & 0x7f) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

fn get_varint(buf: &[u8], pos: &mut usize) -> Option<u32> {
    let mut v: u32 = 0;
    for shift in (0..35).step_by(7) {
        let b = *buf.get(*pos)?;
        *pos += 1;
        v |= u32::from(b & 0x7f).checked_shl(shift)?;
        if b & 0x80 == 0 {
            return Some(v);
        }
    }
    None
}

impl PostingsSection {
    /// Decodes the postings for a term. Malformed data ends the list early.
    pub fn decode(&self, term: usize) -> Vec<Posting> {
        let (Some(&a), Some(&b)) = (self.offsets.get(term), self.offsets.get(term + 1)) else {
            return Vec::new();
        };
        let Some(buf) = self.blob.get(a as usize..b as usize) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let (mut pos, mut doc) = (0, 0u32);
        while pos < buf.len() {
            let (Some(d), Some(t), Some(al), Some(x)) = (
                get_varint(buf, &mut pos),
                get_varint(buf, &mut pos),
                get_varint(buf, &mut pos),
                get_varint(buf, &mut pos),
            ) else {
                break;
            };
            let Some(nd) = doc.checked_add(d) else { break };
            doc = nd;
            out.push(Posting {
                doc,
                tf_title: t,
                tf_alias: al,
                tf_text: x,
            });
        }
        out
    }

    pub fn validate(&self) -> Result<(), IndexError> {
        let n = self.terms.len();
        if self.dfs.len() != n || self.offsets.len() != n + 1 {
            return Err(IndexError::Corrupt("term table size mismatch"));
        }
        if self.offsets.windows(2).any(|w| w[0] > w[1])
            || self.offsets.last().copied().unwrap_or(0) as usize > self.blob.len()
            || self.offsets.first().copied().unwrap_or(0) != 0
        {
            return Err(IndexError::Corrupt("bad posting offsets"));
        }
        if self.terms.windows(2).any(|w| w[0] >= w[1]) {
            return Err(IndexError::Corrupt("terms not sorted"));
        }
        Ok(())
    }
}

pub(crate) fn encode(
    version: u32,
    m: &MetaSection,
    p: &PostingsSection,
    t: &TextSection,
) -> Vec<u8> {
    let enc = |r: Result<Vec<u8>, postcard::Error>| r.expect("postcard encoding of owned data");
    let (m, p, t) = (
        enc(postcard::to_allocvec(m)),
        enc(postcard::to_allocvec(p)),
        enc(postcard::to_allocvec(t)),
    );
    let mut out = Vec::with_capacity(HEADER_LEN + m.len() + p.len() + t.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&version.to_le_bytes());
    for s in [&m, &p, &t] {
        out.extend_from_slice(&(s.len() as u32).to_le_bytes());
    }
    out.extend_from_slice(&m);
    out.extend_from_slice(&p);
    out.extend_from_slice(&t);
    out
}

pub(crate) fn decode(
    version: u32,
    bytes: &[u8],
) -> Result<(MetaSection, PostingsSection, TextSection), IndexError> {
    if bytes.len() < 4 {
        return Err(IndexError::Truncated);
    }
    if &bytes[..4] != MAGIC {
        return Err(IndexError::BadMagic);
    }
    if bytes.len() < HEADER_LEN {
        return Err(IndexError::Truncated);
    }
    let word = |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    if word(4) != version {
        return Err(IndexError::UnsupportedVersion(word(4)));
    }
    let lens = [word(8) as usize, word(12) as usize, word(16) as usize];
    let total = lens
        .iter()
        .try_fold(HEADER_LEN, |a, &l| a.checked_add(l))
        .ok_or(IndexError::Corrupt("section lengths overflow"))?;
    if bytes.len() < total {
        return Err(IndexError::Truncated);
    }
    if bytes.len() > total {
        return Err(IndexError::Corrupt("trailing bytes"));
    }
    let a = HEADER_LEN + lens[0];
    let b = a + lens[1];
    let bad = |_| IndexError::Corrupt("section decode failed");
    let meta: MetaSection = postcard::from_bytes(&bytes[HEADER_LEN..a]).map_err(bad)?;
    let post: PostingsSection = postcard::from_bytes(&bytes[a..b]).map_err(bad)?;
    let text: TextSection = postcard::from_bytes(&bytes[b..total]).map_err(bad)?;
    post.validate()?;
    if text.texts.len() != meta.docs.len() {
        return Err(IndexError::Corrupt("text/doc count mismatch"));
    }
    Ok((meta, post, text))
}
