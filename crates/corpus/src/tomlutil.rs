//! Small, deterministic TOML emitters used by the hand-written entry front
//! matter and the fenced table blocks. Parsing uses the `toml` crate.

use cast_iron_schema::{EntryId, Relation, RelationKind};
use std::fmt::Write;

/// A TOML basic string. Escapes every character that would otherwise end the
/// string or change its bytes. Non-ASCII text stays as UTF-8.
pub fn string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\u{08}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{0c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            ch if (ch as u32) < 0x20 || ch == '\u{7f}' => {
                let _ = write!(out, "\\u{:04X}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// `["a", "b"]`, or `[]` for an empty list. Callers omit empty list keys.
pub fn string_array(values: &[String]) -> String {
    let mut out = String::from("[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&string(value));
    }
    out.push(']');
    out
}

/// One `{ kind = "part", target = "...", label = "..." }` table on a line.
pub fn relation(relation: &Relation) -> String {
    let mut out = String::from("{ kind = ");
    out.push_str(&string(match relation.kind {
        RelationKind::Part => "part",
        RelationKind::Archetype => "archetype",
        RelationKind::Prerequisite => "prerequisite",
    }));
    out.push_str(", target = ");
    out.push_str(&string(relation.target.as_str()));
    if let Some(label) = &relation.label {
        out.push_str(", label = ");
        out.push_str(&string(label));
    }
    out.push_str(" }");
    out
}

/// `"ref/foo"` for an id.
pub fn id(value: &EntryId) -> String {
    string(value.as_str())
}
