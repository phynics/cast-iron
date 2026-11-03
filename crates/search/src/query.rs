use crate::build::cap_text;
use crate::text::{Tok, scan};
use crate::{Filters, Hit, Query, SearchIndex, Snippet, normalize};
use std::collections::BTreeMap;

const DEFAULT_LIMIT: usize = 50;
const MAX_PREFIX_TERMS: usize = 512;
const PREFIX_WEIGHT: f32 = 0.5;
const W_TITLE: f32 = 5.0;
const W_ALIAS: f32 = 3.0;
const W_TEXT: f32 = 1.0;
const K1: f32 = 1.2;
const B: f32 = 0.75;

const BOOST_EXACT_TITLE: f32 = 1000.0;
const BOOST_TITLE_PREFIX: f32 = 500.0;
const BOOST_EXACT_ALIAS: f32 = 450.0;
const BOOST_ALL_IN_TITLE: f32 = 200.0;

const SNIP_BEFORE: usize = 50;
const SNIP_LEN: usize = 200;
const SNIP_SNAP: usize = 20;

#[derive(Clone, Copy, Default)]
struct TokHit {
    score: f32,
    in_title: bool,
}

/// Decides which text tokens count as query matches for highlighting.
struct Matcher {
    exact: Vec<String>,
    prefix: Vec<String>,
}

impl Matcher {
    fn is_match(&self, t: &str) -> bool {
        self.exact.iter().any(|q| q == t) || self.prefix.iter().any(|p| t.starts_with(p.as_str()))
    }

    fn ranges(&self, toks: &[Tok]) -> Vec<(usize, usize)> {
        toks.iter()
            .filter(|t| self.is_match(&t.norm))
            .map(|t| (t.start, t.end))
            .collect()
    }
}

fn contains_ci(list: &[String], v: &str) -> bool {
    list.iter()
        .any(|x| x.eq_ignore_ascii_case(v) || x.to_lowercase() == v.to_lowercase())
}

impl SearchIndex {
    fn passes(&self, ord: usize, f: &Filters) -> bool {
        let d = &self.meta.docs[ord];
        if !f.kinds.is_empty() && !contains_ci(&f.kinds, &d.kind) {
            return false;
        }
        if !f.sources.is_empty() && !contains_ci(&f.sources, &d.source) {
            return false;
        }
        let mut fields: Vec<String> = f.facets.iter().map(|(k, _)| k.to_lowercase()).collect();
        fields.sort();
        fields.dedup();
        fields.iter().all(|field| {
            d.facets.iter().any(|(k, v)| {
                k.to_lowercase() == *field
                    && f.facets.iter().any(|(fk, fv)| {
                        fk.to_lowercase() == *field && fv.to_lowercase() == v.to_lowercase()
                    })
            })
        })
    }

    /// (term index, weight) pairs for one query token.
    fn expand(&self, q: &str, allow_prefix_always: bool) -> (Vec<(usize, f32)>, bool) {
        let terms = &self.postings.terms;
        let mut out = Vec::new();
        let exact = terms.binary_search_by(|t| t.as_str().cmp(q)).ok();
        if let Some(i) = exact {
            out.push((i, 1.0));
        }
        let use_prefix = allow_prefix_always || exact.is_none();
        if use_prefix {
            let start = terms.partition_point(|t| t.as_str() < q);
            out.extend(
                terms[start..]
                    .iter()
                    .take_while(|t| t.starts_with(q))
                    .enumerate()
                    .map(|(k, _)| (start + k, PREFIX_WEIGHT))
                    .filter(|(i, _)| Some(*i) != exact)
                    .take(MAX_PREFIX_TERMS),
            );
        }
        (out, use_prefix)
    }

    fn token_hits(&self, expansion: &[(usize, f32)]) -> BTreeMap<u32, TokHit> {
        let n = self.meta.docs.len() as f32;
        let m = &self.meta;
        let avg = |a: f32| a.max(1.0);
        let mut map: BTreeMap<u32, TokHit> = BTreeMap::new();
        for &(ti, weight) in expansion {
            let df = self.postings.dfs[ti] as f32;
            let idf = (1.0 + (n - df + 0.5) / (df + 0.5)).ln();
            for p in self.postings.decode(ti) {
                let Some(d) = m.docs.get(p.doc as usize) else {
                    continue;
                };
                let part = |tf: u32, len: u32, a: f32, w: f32| {
                    if tf == 0 {
                        return 0.0;
                    }
                    let tf = tf as f32;
                    w * tf * (K1 + 1.0) / (tf + K1 * (1.0 - B + B * len as f32 / avg(a)))
                };
                let s = part(p.tf_title, d.len_title, m.avg_title, W_TITLE)
                    + part(p.tf_alias, d.len_alias, m.avg_alias, W_ALIAS)
                    + part(p.tf_text, d.len_text, m.avg_text, W_TEXT);
                let e = map.entry(p.doc).or_default();
                e.score += s * idf * weight;
                e.in_title |= p.tf_title > 0;
            }
        }
        map
    }

    pub fn search(&self, query: &Query) -> Vec<Hit> {
        let limit = if query.limit == 0 {
            DEFAULT_LIMIT
        } else {
            query.limit
        };
        let qtokens: Vec<String> = scan(&query.text).into_iter().map(|t| t.norm).collect();
        if qtokens.is_empty() {
            if query.filters.is_empty() {
                return Vec::new();
            }
            let mut ords: Vec<(String, &str, u32)> = (0..self.meta.docs.len())
                .filter(|&i| self.passes(i, &query.filters))
                .map(|i| {
                    (
                        normalize(&self.meta.docs[i].title),
                        self.meta.docs[i].id.as_str(),
                        i as u32,
                    )
                })
                .collect();
            ords.sort();
            let m = Matcher {
                exact: vec![],
                prefix: vec![],
            };
            return ords
                .into_iter()
                .take(limit)
                .map(|(_, _, o)| self.make_hit(o, 0.0, &m))
                .collect();
        }

        let last = qtokens.len() - 1;
        let mut per_token = Vec::with_capacity(qtokens.len());
        let mut matcher = Matcher {
            exact: vec![],
            prefix: vec![],
        };
        for (i, q) in qtokens.iter().enumerate() {
            let (exp, prefixed) = self.expand(q, i == last);
            if exp.is_empty() {
                return Vec::new();
            }
            matcher.exact.push(q.clone());
            if prefixed {
                matcher.prefix.push(q.clone());
            }
            per_token.push(self.token_hits(&exp));
        }
        let qn = qtokens.join(" ");
        let (first, rest) = per_token.split_first().expect("non-empty");
        let mut scored: Vec<(u32, f32)> = Vec::new();
        for (&doc, h0) in first {
            if !self.passes(doc as usize, &query.filters) {
                continue;
            }
            let mut score = h0.score;
            let mut all_title = h0.in_title;
            let mut ok = true;
            for m in rest {
                match m.get(&doc) {
                    Some(h) => {
                        score += h.score;
                        all_title &= h.in_title;
                    }
                    None => {
                        ok = false;
                        break;
                    }
                }
            }
            if !ok {
                continue;
            }
            let d = &self.meta.docs[doc as usize];
            let nt = normalize(&d.title);
            let mut boost = 0.0f32;
            if nt == qn {
                boost = BOOST_EXACT_TITLE;
            } else if nt.starts_with(&qn) {
                boost = BOOST_TITLE_PREFIX;
            } else if d.aliases.iter().any(|a| normalize(a) == qn) {
                boost = BOOST_EXACT_ALIAS;
            } else if all_title {
                boost = BOOST_ALL_IN_TITLE;
            }
            scored.push((doc, score + boost));
        }
        scored.sort_by(|a, b| {
            let (da, db) = (&self.meta.docs[a.0 as usize], &self.meta.docs[b.0 as usize]);
            b.1.total_cmp(&a.1)
                .then_with(|| da.title.cmp(&db.title))
                .then_with(|| da.id.cmp(&db.id))
        });
        scored
            .into_iter()
            .take(limit)
            .map(|(o, s)| self.make_hit(o, s, &matcher))
            .collect()
    }

    fn make_hit(&self, ord: u32, score: f32, m: &Matcher) -> Hit {
        let d = &self.meta.docs[ord as usize];
        let title_matches = m.ranges(&scan(&d.title));
        let body = self
            .texts
            .texts
            .get(ord as usize)
            .map_or("", |s| cap_text(s));
        Hit {
            ord,
            id: d.id.clone(),
            title: d.title.clone(),
            kind: d.kind.clone(),
            source: d.source.clone(),
            score,
            title_matches,
            snippet: snippet(body, m),
        }
    }
}

fn snippet(body: &str, m: &Matcher) -> Snippet {
    let ranges = m.ranges(&scan(body));
    let mut start = 0;
    if let Some(&(first, _)) = ranges.first() {
        start = body[..first]
            .char_indices()
            .rev()
            .nth(SNIP_BEFORE - 1)
            .map_or(0, |(i, _)| i);
        if start > 0 {
            // Snap forward to a word start, within a small slack.
            let tail = &body[start..first];
            if let Some((i, c)) = tail
                .char_indices()
                .take(SNIP_SNAP)
                .find(|(_, c)| c.is_whitespace())
            {
                start += i + c.len_utf8();
            }
        }
    }
    let rest = &body[start..];
    let mut end = rest
        .char_indices()
        .nth(SNIP_LEN)
        .map_or(body.len(), |(i, _)| start + i);
    if end < body.len() {
        let head = &body[start..end];
        if let Some((i, _)) = head
            .char_indices()
            .rev()
            .take(SNIP_SNAP)
            .find(|(_, c)| c.is_whitespace())
            && start + i > start
        {
            end = start + i;
        }
    }
    let (pre, post) = (
        if start > 0 { "…" } else { "" },
        if end < body.len() { "…" } else { "" },
    );
    let shift = pre.len();
    let matches = ranges
        .into_iter()
        .filter(|&(a, b)| a >= start && b <= end)
        .map(|(a, b)| (a - start + shift, b - start + shift))
        .collect();
    Snippet {
        text: format!("{pre}{}{post}", &body[start..end]),
        matches,
    }
}
