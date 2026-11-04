//! Expansion of the pinned source's record aliases.
//!
//! The source schema defines `copyof` as an entry copied from another key in the
//! same JSON file, with the copy's name overriding the original name. A
//! `redirect` is a route alias; expansion follows it to its destination. The
//! inventory remains authoritative for the alias record itself.

use crate::pf_data::InputRecord;
use serde_json::Value;
use std::collections::BTreeSet;

/// Expand a record alias against the complete inventory.
///
/// Copy and redirect targets must resolve to one exact `(protocol, key)` pair.
/// A copy inherits the target JSON and overrides only its `name`, as described
/// by the source schema and prebuild copy behavior. Redirects resolve to the
/// destination JSON; their distinct route identity remains on `InputRecord`.
/// Compilation recipes are deliberately rejected until their selectors and
/// Markdown assembly are implemented. External tables are expanded in-place
/// in descriptions; the original table definitions remain in the returned
/// JSON for callers that need the source inventory.
pub fn expand(record: &InputRecord, all: &[InputRecord]) -> Result<Value, String> {
    let mut visiting = BTreeSet::new();
    expand_inner(record, all, &mut visiting)
}

fn expand_inner(
    record: &InputRecord,
    all: &[InputRecord],
    visiting: &mut BTreeSet<(String, String, String)>,
) -> Result<Value, String> {
    let protocol = record
        .protocol
        .as_deref()
        .ok_or_else(|| format!("{}#{} has no source protocol", record.path, record.key))?;
    let identity = (protocol.to_owned(), record.path.clone(), record.key.clone());
    if !visiting.insert(identity.clone()) {
        return Err(format!(
            "record alias cycle at {}#{}",
            record.path, record.key
        ));
    }

    let result = (|| {
        if let Some(target_key) = record.raw.get("copyof").and_then(Value::as_str) {
            let target = resolve(record, target_key, all)?;
            let mut expanded = expand_inner(target, all, visiting)?;
            let object = expanded.as_object_mut().ok_or_else(|| {
                format!(
                    "copy target {}#{} is not an object",
                    target.path, target.key
                )
            })?;
            object.insert("name".into(), Value::String(record.name.clone()));
            return Ok(expanded);
        }
        if let Some(target_key) = record.raw.get("redirect").and_then(Value::as_str) {
            let target = resolve(record, target_key, all)?;
            return expand_inner(target, all, visiting);
        }
        if record.raw.get("compileFrom").is_some() {
            return Err(format!(
                "{}#{} uses compileFrom; recipe expansion is not implemented",
                record.path, record.key
            ));
        }
        // Return a clone so callers cannot mutate the source inventory.
        let mut expanded = record.raw.clone();
        // JSON objects are ordered by serde_json's map implementation; forcing
        // an object here also gives a precise error for malformed source rows.
        if !matches!(expanded, Value::Object(_)) {
            return Err(format!(
                "{}#{} is not a JSON object",
                record.path, record.key
            ));
        }
        if let Value::Object(map) = &mut expanded {
            // `name` is the source field for entry names. Some page records use
            // `title`; leave that metadata untouched for non-copy records.
            if map.get("name").is_some_and(Value::is_null) {
                map.remove("name");
            }
            let definitions = map.get("tables").cloned();
            if let Some(description) = map.get_mut("description") {
                expand_tables(description, definitions.as_ref(), record)?;
            }
        }
        Ok(expanded)
    })();

    visiting.remove(&identity);
    result
}

fn expand_tables(
    description: &mut Value,
    definitions: Option<&Value>,
    record: &InputRecord,
) -> Result<(), String> {
    let Some(lines) = description.as_array_mut() else {
        return Ok(());
    };
    let tables = definitions
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for line in lines {
        let Some(text) = line.as_str() else {
            continue;
        };
        let mut out = text.to_owned();
        let mut refs = Vec::new();
        let mut rest = text;
        let mut offset = 0;
        while let Some(start) = rest.find("‹table") {
            let tail = &rest[start + "‹table".len()..];
            let Some(end) = tail.find("›") else {
                break;
            };
            let index = &tail[..end];
            if !index.is_empty() && index.bytes().all(|b| b.is_ascii_digit()) {
                refs.push((
                    offset + start,
                    offset + start + "‹table".len() + end + "›".len(),
                    index.parse::<usize>().map_err(|_| {
                        format!("{}#{} has invalid table reference", record.path, record.key)
                    })?,
                ));
            }
            let consumed = start + "‹table".len() + end + "›".len();
            offset += consumed;
            rest = &rest[consumed..];
        }
        for (start, end, index) in refs.into_iter().rev() {
            let table = tables.get(index).ok_or_else(|| {
                format!(
                    "{}#{} references missing table {index}",
                    record.path, record.key
                )
            })?;
            let rendered = render_table(table, record)?;
            out.replace_range(start..end, &rendered);
        }
        if out != text {
            *line = Value::String(out);
        }
    }
    Ok(())
}

fn render_table(table: &Value, record: &InputRecord) -> Result<String, String> {
    let fail = || {
        format!(
            "{}#{} has unsupported or malformed table definition",
            record.path, record.key
        )
    };
    let columns = table
        .get("columns")
        .and_then(Value::as_array)
        .ok_or_else(fail)?;
    let rows = table
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(fail)?;
    let mut output = String::new();
    output.push('|');
    for column in columns {
        let header = column
            .get("header")
            .and_then(Value::as_str)
            .ok_or_else(fail)?;
        output.push_str(&format!(" {} |", md_cell(header)));
    }
    output.push('\n');
    output.push('|');
    for _ in columns {
        output.push_str(" --- |");
    }
    for row in rows {
        let cells = row.as_array().ok_or_else(fail)?;
        if cells.len() != columns.len() {
            return Err(fail());
        }
        output.push('\n');
        output.push('|');
        for (cell, column) in cells.iter().zip(columns) {
            output.push(' ');
            output.push_str(&render_cell(
                cell,
                column,
                table
                    .get("nullValue")
                    .and_then(Value::as_str)
                    .unwrap_or("—"),
                record,
            )?);
            output.push_str(" |");
        }
    }
    Ok(output)
}

fn render_cell(
    cell: &Value,
    column: &Value,
    null_value: &str,
    record: &InputRecord,
) -> Result<String, String> {
    let fail = || {
        format!(
            "{}#{} has unsupported table cell value",
            record.path, record.key
        )
    };
    let column_type = column.get("type").and_then(Value::as_str).unwrap_or("");
    if !["", "gp", "gp+", "lbs", "lbs+", "bonus", "num", "pct"].contains(&column_type) {
        return Err(format!(
            "{}#{} has unsupported table column transform {column_type:?}",
            record.path, record.key
        ));
    }
    if cell.is_null() || cell.as_str() == Some("~~") {
        return Ok(md_cell(null_value));
    }
    let (value, sortable) = if let Some(obj) = cell.as_object() {
        let data = obj.get("data").unwrap_or(&Value::Null);
        if obj.keys().any(|k| k != "sort" && k != "data") {
            return Err(fail());
        }
        (data, true)
    } else {
        (cell, false)
    };
    let display = if let Some(parts) = value.as_array() {
        if !column_type.is_empty() {
            return Err(format!(
                "{}#{} has linked value for table column transform {column_type:?}",
                record.path, record.key
            ));
        }
        if !(2..=3).contains(&parts.len()) {
            return Err(fail());
        }
        let text = parts[0].as_str().ok_or_else(fail)?;
        let target = parts[1].as_str().ok_or_else(fail)?;
        let protocol = parts
            .get(2)
            .and_then(Value::as_str)
            .or_else(|| column.get("link").and_then(Value::as_str))
            .ok_or_else(fail)?;
        format!("[{}]({}/{})", md_cell(text), protocol, target)
    } else if let Some(s) = value.as_str() {
        if !column_type.is_empty() {
            return Err(format!(
                "{}#{} has non-numeric value for table column transform {column_type:?}",
                record.path, record.key
            ));
        }
        md_cell(s)
    } else if let Some(n) = value.as_f64() {
        match column_type {
            "gp" | "gp+" => format_gp(n, column["type"] == "gp+", sortable),
            "lbs" | "lbs+" => format!(
                "{}{} lb{}.",
                if column["type"] == "lbs+" && n >= 0.0 {
                    "+"
                } else {
                    ""
                },
                n,
                if n == 1.0 { "" } else { "s" }
            ),
            "bonus" => format!("{}{n}", if n >= 0.0 { "+" } else { "" }),
            "pct" => format!("{n}%"),
            "" | "num" => n.to_string(),
            _ => return Err(fail()),
        }
    } else {
        return Err(fail());
    };
    Ok(display)
}

fn format_gp(gp: f64, adjustment: bool, sortable: bool) -> String {
    if sortable {
        return gp.to_string();
    }
    let sign = if adjustment && gp >= 0.0 { "+" } else { "" };
    if gp.fract() == 0.0 {
        format!("{sign}{gp} gp")
    } else if (gp * 10.0).fract() == 0.0 {
        format!("{sign}{} sp", gp * 10.0)
    } else {
        format!("{sign}{} cp", gp * 100.0)
    }
}

fn md_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', "<br>")
}

fn resolve<'a>(
    from: &InputRecord,
    key: &str,
    all: &'a [InputRecord],
) -> Result<&'a InputRecord, String> {
    let protocol = from
        .protocol
        .as_deref()
        .ok_or_else(|| format!("{}#{} has no source protocol", from.path, from.key))?;
    let mut matches = all.iter().filter(|candidate| {
        candidate.protocol.as_deref() == Some(protocol) && candidate.key == key
    });
    let target = matches.next().ok_or_else(|| {
        format!(
            "{}#{} references missing {} target {:?}",
            from.path,
            from.key,
            if from.raw.get("copyof").is_some() {
                "copyof"
            } else {
                "redirect"
            },
            key
        )
    })?;
    if matches.next().is_some() {
        return Err(format!(
            "{}#{} has ambiguous target protocol {:?}, key {:?}",
            from.path, from.key, protocol, key
        ));
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn record(protocol: &str, path: &str, key: &str, name: &str, raw: Value) -> InputRecord {
        InputRecord {
            protocol: Some(protocol.into()),
            path: path.into(),
            key: key.into(),
            name: name.into(),
            shape: "description".into(),
            publications: vec!["Advanced Player's Guide".into()],
            raw,
        }
    }

    #[test]
    fn copies_inherit_content_but_keep_the_alias_name() {
        let source = record(
            "spell",
            "json/spells.json",
            "produce_flame",
            "Produce Flame",
            json!({"name":"Produce Flame", "sources":["Core Rulebook"], "description":["A flame."]}),
        );
        let copy = record(
            "spell",
            "json/spells15.json",
            "pale_flame",
            "Pale Flame",
            json!({"name":"Pale Flame", "copyof":"produce_flame"}),
        );
        let expanded = expand(&copy, &[source, copy.clone()]).unwrap();
        assert_eq!(expanded["name"], "Pale Flame");
        assert_eq!(expanded["description"][0], "A flame.");
        assert_eq!(expanded["sources"][0], "Core Rulebook");
    }

    #[test]
    fn redirects_follow_the_destination_without_copying_route_identity() {
        let destination = record(
            "spell",
            "json/spells.json",
            "absorb_rune_1",
            "Absorb Rune I",
            json!({"name":"Absorb Rune I", "description":["Destination."]}),
        );
        let alias = record(
            "spell",
            "json/spells.json",
            "absorb_rune_i",
            "",
            json!({"redirect":"absorb_rune_1"}),
        );
        let expanded = expand(&alias, &[destination, alias.clone()]).unwrap();
        assert_eq!(expanded["name"], "Absorb Rune I");
        assert_eq!(expanded["description"][0], "Destination.");
    }

    #[test]
    fn rejects_alias_cycles_and_missing_targets() {
        let a = record(
            "feat",
            "json/feat.json",
            "a",
            "A",
            json!({"name":"A", "copyof":"b"}),
        );
        let b = record(
            "feat",
            "json/feat.json",
            "b",
            "B",
            json!({"name":"B", "copyof":"a"}),
        );
        assert!(
            expand(&a, &[a.clone(), b.clone()])
                .unwrap_err()
                .contains("cycle")
        );
        let missing = record(
            "feat",
            "json/feat.json",
            "missing",
            "Missing",
            json!({"redirect":"absent"}),
        );
        assert!(
            expand(&missing, std::slice::from_ref(&missing))
                .unwrap_err()
                .contains("missing redirect target")
        );
    }

    #[test]
    fn compile_recipes_fail_explicitly_and_unknown_data_is_retained() {
        let recipe = record(
            "ability",
            "json/ability.json",
            "recipe",
            "Recipe",
            json!({"name":"Recipe", "compileFrom":{"source":"ragepower", "targets":[]}}),
        );
        assert!(
            expand(&recipe, std::slice::from_ref(&recipe))
                .unwrap_err()
                .contains("compileFrom")
        );
        let table = record(
            "main",
            "json/main.json",
            "table",
            "Table",
            json!({"title":"Table", "tables":[{"id":"sample"}]}),
        );
        assert_eq!(
            expand(&table, std::slice::from_ref(&table)).unwrap()["tables"][0]["id"],
            "sample"
        );
    }

    #[test]
    fn external_tables_expand_in_place_with_order_and_cell_values() {
        let class = record(
            "class",
            "json/classes.json",
            "sample",
            "Sample",
            json!({
                "name":"Sample",
                "description":["Before", "‹table0›", "After"],
                "tables":[{
                    "id":"sample",
                    "columns":[{"header":"Level", "type":"num"},{"header":"Feature", "link":"feat"},{"header":"Cost", "type":"gp"}],
                    "initialColumn":0,
                    "data":[[1,["First feature","first_feature"],2.5],[2,null,3]],
                    "nullValue":"—"
                }]
            }),
        );
        let expanded = expand(&class, std::slice::from_ref(&class)).unwrap();
        let description = expanded["description"].as_array().unwrap();
        assert_eq!(description[0], "Before");
        let table = description[1].as_str().unwrap();
        assert!(table.contains("| Level | Feature | Cost |"));
        assert!(table.contains("| 1 | [First feature](feat/first_feature) | 25 sp |"));
        assert!(table.contains("| 2 | — | 3 gp |"));
        assert_eq!(description[2], "After");
        assert_eq!(expanded["tables"][0]["data"][0][0], 1);
    }

    #[test]
    fn external_table_references_fail_when_no_definition_exists() {
        let class = record(
            "class",
            "json/classes.json",
            "sample",
            "Sample",
            json!({
                "name":"Sample", "description":["‹table1›"], "tables":[]
            }),
        );
        assert!(
            expand(&class, std::slice::from_ref(&class))
                .unwrap_err()
                .contains("missing table 1")
        );
    }

    #[test]
    fn pinned_class_fixture_expands_its_external_table() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.cache/sources/pathfinder-data-1-e/json/classes.json");
        if !path.is_file() {
            eprintln!("skipping: the pinned primary source is not present");
            return;
        }
        let source: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let raw = source
            .as_object()
            .unwrap()
            .values()
            .find(|entry| {
                entry.get("tables").is_some()
                    && entry
                        .get("description")
                        .and_then(Value::as_array)
                        .is_some_and(|lines| {
                            lines.iter().any(|line| {
                                line.as_str().is_some_and(|line| line.contains("‹table"))
                            })
                        })
            })
            .unwrap()
            .clone();
        let key = raw
            .get("link")
            .and_then(Value::as_str)
            .unwrap_or("fixture")
            .to_owned();
        let name = raw
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("Fixture")
            .to_owned();
        let class = record("class", "json/classes.json", &key, &name, raw);
        let expanded = expand(&class, std::slice::from_ref(&class)).unwrap();
        let description = expanded["description"].as_array().unwrap();
        assert!(description.iter().any(|line| {
            line.as_str()
                .is_some_and(|line| line.starts_with('|') && line.contains(" --- |"))
        }));
        assert!(
            description
                .iter()
                .all(|line| line.as_str().is_none_or(|line| !line.contains("‹table")))
        );
    }
}
