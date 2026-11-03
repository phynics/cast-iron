use crate::store::{DocMeta, MetaSection, PostingsSection, TextSection, put_varint};
use crate::text::scan;
use crate::{DocInput, SearchIndex};
use std::collections::BTreeMap;

pub(crate) const TEXT_CAP: usize = 6 * 1024;

/// Collects documents and builds a deterministic [`SearchIndex`].
#[derive(Default)]
pub struct IndexBuilder {
    docs: Vec<DocInput>,
}

impl IndexBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, doc: DocInput) {
        self.docs.push(doc);
    }

    pub fn build(mut self) -> SearchIndex {
        self.docs.sort_by(|a, b| {
            (
                &a.id, &a.title, &a.kind, &a.source, &a.section, &a.text, &a.aliases, &a.facets,
            )
                .cmp(&(
                    &b.id, &b.title, &b.kind, &b.source, &b.section, &b.text, &b.aliases, &b.facets,
                ))
        });
        // term -> postings (doc, tf_title, tf_alias, tf_text), docs ascending.
        let mut table: BTreeMap<String, Vec<(u32, u32, u32, u32)>> = BTreeMap::new();
        let mut metas = Vec::with_capacity(self.docs.len());
        let mut texts = Vec::with_capacity(self.docs.len());
        let (mut st, mut sa, mut sx) = (0u64, 0u64, 0u64);
        for (ord, d) in self.docs.into_iter().enumerate() {
            let ord = ord as u32;
            let mut tfs: BTreeMap<String, [u32; 3]> = BTreeMap::new();
            let mut lens = [0u32; 3];
            let mut add = |field: usize, s: &str| {
                for t in scan(s) {
                    tfs.entry(t.norm).or_default()[field] += 1;
                    lens[field] += 1;
                }
            };
            add(0, &d.title);
            for a in &d.aliases {
                add(1, a);
            }
            add(2, &d.text);
            for (term, tf) in tfs {
                table
                    .entry(term)
                    .or_default()
                    .push((ord, tf[0], tf[1], tf[2]));
            }
            st += u64::from(lens[0]);
            sa += u64::from(lens[1]);
            sx += u64::from(lens[2]);
            texts.push(cap_text(&d.text).to_string());
            metas.push(DocMeta {
                id: d.id,
                title: d.title,
                aliases: d.aliases,
                kind: d.kind,
                source: d.source,
                section: d.section,
                facets: d.facets,
                len_title: lens[0],
                len_alias: lens[1],
                len_text: lens[2],
            });
        }
        let n = metas.len().max(1) as f64;
        let mut p = PostingsSection {
            offsets: vec![0],
            ..Default::default()
        };
        for (term, list) in table {
            let mut prev = 0;
            for (doc, a, b, c) in &list {
                put_varint(&mut p.blob, doc - prev);
                put_varint(&mut p.blob, *a);
                put_varint(&mut p.blob, *b);
                put_varint(&mut p.blob, *c);
                prev = *doc;
            }
            p.terms.push(term);
            p.dfs.push(list.len() as u32);
            p.offsets.push(p.blob.len() as u32);
        }
        let meta = MetaSection {
            docs: metas,
            avg_title: (st as f64 / n) as f32,
            avg_alias: (sa as f64 / n) as f32,
            avg_text: (sx as f64 / n) as f32,
        };
        SearchIndex::from_parts(meta, p, TextSection { texts })
    }
}

pub(crate) fn cap_text(s: &str) -> &str {
    if s.len() <= TEXT_CAP {
        return s;
    }
    let mut end = TEXT_CAP;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
