//! Normalization and tokenization. A single scanner produces tokens together
//! with byte ranges into the original text, so highlights stay valid.

/// A normalized token and its byte range in the original text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tok {
    pub norm: String,
    pub start: usize,
    pub end: usize,
}

enum Class {
    /// Part of a word but contributes nothing (apostrophes, combining marks).
    Skip,
    Sep,
    Word,
}

fn fold_lower(c: char) -> Option<&'static str> {
    Some(match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
        'æ' => "ae",
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => "c",
        'ď' | 'đ' | 'ð' => "d",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e",
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => "g",
        'ĥ' | 'ħ' => "h",
        'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => "i",
        'ĵ' => "j",
        'ķ' => "k",
        'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => "l",
        'ñ' | 'ń' | 'ņ' | 'ň' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => "o",
        'œ' => "oe",
        'ŕ' | 'ŗ' | 'ř' => "r",
        'ś' | 'ŝ' | 'ş' | 'š' => "s",
        'ß' => "ss",
        'ţ' | 'ť' | 'ŧ' => "t",
        'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => "u",
        'ŵ' => "w",
        'ý' | 'ÿ' | 'ŷ' => "y",
        'ź' | 'ż' | 'ž' => "z",
        _ => return None,
    })
}

fn classify(c: char, buf: &mut String) -> Class {
    match c {
        '\'' | '\u{2019}' | '\u{2018}' | '\u{02BC}' | '`' | '\u{00B4}' => return Class::Skip,
        '\u{0300}'..='\u{036F}' => return Class::Skip,
        _ => {}
    }
    let mut any = false;
    for l in c.to_lowercase() {
        if let Some(s) = fold_lower(l) {
            buf.push_str(s);
            any = true;
        } else if l.is_alphanumeric() {
            buf.push(l);
            any = true;
        }
    }
    if any { Class::Word } else { Class::Sep }
}

pub(crate) fn scan(text: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut start, mut end) = (0, 0);
    let mut in_word = false;
    for (i, c) in text.char_indices() {
        let before = cur.len();
        match classify(c, &mut cur) {
            Class::Word => {
                if !in_word {
                    in_word = true;
                    start = i;
                }
                end = i + c.len_utf8();
            }
            Class::Skip => {
                if in_word {
                    end = i + c.len_utf8();
                }
                cur.truncate(before);
            }
            Class::Sep => {
                cur.truncate(before);
                if in_word {
                    out.push(Tok {
                        norm: std::mem::take(&mut cur),
                        start,
                        end,
                    });
                    in_word = false;
                }
            }
        }
    }
    if in_word {
        out.push(Tok {
            norm: cur,
            start,
            end,
        });
    }
    out
}

/// Lowercase, fold diacritics and quotes, collapse punctuation to single spaces.
pub fn normalize(text: &str) -> String {
    let mut s = String::with_capacity(text.len());
    for t in scan(text) {
        if !s.is_empty() {
            s.push(' ');
        }
        s.push_str(&t.norm);
    }
    s
}

/// Tokens of [`normalize`].
pub fn tokenize(text: &str) -> Vec<String> {
    scan(text).into_iter().map(|t| t.norm).collect()
}
