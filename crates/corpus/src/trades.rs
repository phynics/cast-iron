//! The trades of an archetype or class option, in both corpus formats.
//!
//! Format 2 (#450) writes one flat `trades` list, one inline table per trade:
//!
//! ```toml
//! trades = [
//!   { replaces = "Trap sense (Ex)", feature = "trap-sense" },
//!   { alters = "Bonus feats", feature = "bonus-feats", class = "ref/class/fighter", levels = [4, 8] },
//!   { replaces = "Acidic ray", feature = "bloodline-powers", power = "Warp Touch" },
//! ]
//! ```
//!
//! Format 1 wrote four lists: `replaces`, `alters`, and the heading anchors `replaces_features`
//! and `alters_features`. The format-1 reader lowers them to the same [`Trade`]s. A feature list
//! lines up with its printed list only when both have the same length; then each anchor goes to
//! the printed name at the same position. Any other non-empty feature list cannot say which name
//! each anchor belongs to, so the trades stay unresolved and the reader keeps the anchors in
//! [`LegacyFeatureLists`]; the format-1 writer puts them back, so `fmt` leaves the file as it
//! was.

use crate::error::{Error, Result};
use crate::tomlutil;
use cast_iron_schema::{EntryId, FeatureRef, LegacyFeatureLists, Trade, TradeOp, printed_trades};
use std::path::Path;

/// The keys of the trade fields in each format, after `kind` (and `group` for a class option).
pub const FORMAT_1_KEYS: [&str; 4] = ["replaces", "alters", "replaces_features", "alters_features"];
pub const FORMAT_2_KEYS: [&str; 1] = ["trades"];

/// The keys a trade-bearing `[fields]` table may hold in `format`, besides `kind` and `group`.
pub fn field_keys(format: u32) -> &'static [&'static str] {
    if format == 1 {
        &FORMAT_1_KEYS
    } else {
        &FORMAT_2_KEYS
    }
}

fn list(path: &Path, table: &toml::Table, key: &str, line: usize) -> Result<Vec<String>> {
    let Some(value) = table.get(key) else {
        return Ok(Vec::new());
    };
    let toml::Value::Array(values) = value else {
        return Err(Error::new(
            path,
            line,
            format!("`{key}` must be an array of strings"),
        ));
    };
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| Error::new(path, line, format!("`{key}` must hold strings only")))
        })
        .collect()
}

/// Reads the trade keys of a `[fields]` table. `line` finds the line of a key.
pub fn read_trades(
    path: &Path,
    table: &toml::Table,
    format: u32,
    line: &dyn Fn(&str) -> usize,
) -> Result<(Vec<Trade>, LegacyFeatureLists)> {
    if format == 1 {
        read_format_1(path, table, line)
    } else {
        Ok((
            read_format_2(path, table, line("trades"))?,
            Default::default(),
        ))
    }
}

fn read_format_1(
    path: &Path,
    table: &toml::Table,
    line: &dyn Fn(&str) -> usize,
) -> Result<(Vec<Trade>, LegacyFeatureLists)> {
    let mut trades = Vec::new();
    let mut legacy = LegacyFeatureLists::default();
    for (op, names_key, features_key) in [
        (TradeOp::Replaces, "replaces", "replaces_features"),
        (TradeOp::Alters, "alters", "alters_features"),
    ] {
        let names = list(path, table, names_key, line(names_key))?;
        let anchors = list(path, table, features_key, line(features_key))?;
        let aligned = !anchors.is_empty() && anchors.len() == names.len();
        if !anchors.is_empty() && !aligned {
            match op {
                TradeOp::Replaces => legacy.replaces = anchors.clone(),
                TradeOp::Alters => legacy.alters = anchors.clone(),
            }
        }
        for (index, printed) in names.into_iter().enumerate() {
            let feature = aligned.then(|| FeatureRef {
                anchor: anchors[index].clone(),
                ..Default::default()
            });
            trades.push(Trade {
                op,
                printed,
                feature,
            });
        }
    }
    Ok((trades, legacy))
}

fn read_format_2(path: &Path, table: &toml::Table, line: usize) -> Result<Vec<Trade>> {
    let Some(value) = table.get("trades") else {
        return Ok(Vec::new());
    };
    let toml::Value::Array(rows) = value else {
        return Err(Error::new(
            path,
            line,
            "`trades` must be an array of tables, one per trade",
        ));
    };
    let mut out = Vec::new();
    for row in rows {
        let row = row
            .as_table()
            .ok_or_else(|| Error::new(path, line, "`trades` must hold tables"))?;
        out.push(read_trade(path, row, line)?);
    }
    Ok(out)
}

fn read_trade(path: &Path, row: &toml::Table, line: usize) -> Result<Trade> {
    const KEYS: [&str; 6] = ["replaces", "alters", "feature", "class", "levels", "power"];
    for key in row.keys() {
        if !KEYS.contains(&key.as_str()) {
            return Err(Error::new(
                path,
                line,
                format!(
                    "unexpected key `{key}` in a trade; a trade has `replaces` or `alters`, and optionally `feature`, `class`, `levels` and `power`"
                ),
            ));
        }
    }
    let string = |key: &str| -> Result<Option<String>> {
        match row.get(key) {
            None => Ok(None),
            Some(toml::Value::String(value)) => Ok(Some(value.clone())),
            Some(_) => Err(Error::new(
                path,
                line,
                format!("`{key}` in a trade must be a string"),
            )),
        }
    };
    let (op, printed) = match (string("replaces")?, string("alters")?) {
        (Some(printed), None) => (TradeOp::Replaces, printed),
        (None, Some(printed)) => (TradeOp::Alters, printed),
        _ => {
            return Err(Error::new(
                path,
                line,
                "a trade needs exactly one of `replaces` and `alters`, with the printed feature name",
            ));
        }
    };
    if printed.trim().is_empty() {
        return Err(Error::new(path, line, "a trade needs a printed name"));
    }
    let anchor = string("feature")?;
    let class = string("class")?;
    let power = string("power")?;
    let levels = match row.get("levels") {
        None => Vec::new(),
        Some(toml::Value::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_integer()
                    .and_then(|level| u8::try_from(level).ok())
                    .filter(|level| *level >= 1)
                    .ok_or_else(|| {
                        Error::new(
                            path,
                            line,
                            "`levels` in a trade must hold whole numbers from 1 to 255",
                        )
                    })
            })
            .collect::<Result<Vec<u8>>>()?,
        Some(_) => {
            return Err(Error::new(
                path,
                line,
                "`levels` in a trade must be an array of numbers",
            ));
        }
    };
    let Some(anchor) = anchor else {
        if class.is_some() || power.is_some() || !levels.is_empty() {
            return Err(Error::new(
                path,
                line,
                format!(
                    "the trade `{printed}` has `class`, `levels` or `power` but no `feature`; they narrow a feature"
                ),
            ));
        }
        return Ok(Trade::unresolved(op, printed));
    };
    let class = class
        .map(|value| EntryId::parse(value).map_err(|e| Error::new(path, line, e.to_string())))
        .transpose()?;
    Ok(Trade {
        op,
        printed,
        feature: Some(FeatureRef {
            anchor,
            class,
            levels,
            power,
        }),
    })
}

/// Writes the trade keys of a `[fields]` table in `format`.
pub fn write_trades(out: &mut String, trades: &[Trade], legacy: &LegacyFeatureLists, format: u32) {
    if format == 1 {
        write_format_1(out, trades, legacy);
    } else {
        write_format_2(out, trades);
    }
}

fn write_array(out: &mut String, key: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    out.push_str(key);
    out.push_str(" = ");
    out.push_str(&tomlutil::string_array(values));
    out.push('\n');
}

fn write_format_1(out: &mut String, trades: &[Trade], legacy: &LegacyFeatureLists) {
    let anchors = |op: TradeOp, kept: &[String]| -> Vec<String> {
        if !kept.is_empty() {
            return kept.to_vec();
        }
        trades
            .iter()
            .filter(|trade| trade.op == op)
            .filter_map(|trade| trade.feature.as_ref().map(|feature| feature.anchor.clone()))
            .collect()
    };
    write_array(out, "replaces", &printed_trades(trades, TradeOp::Replaces));
    write_array(out, "alters", &printed_trades(trades, TradeOp::Alters));
    write_array(
        out,
        "replaces_features",
        &anchors(TradeOp::Replaces, &legacy.replaces),
    );
    write_array(
        out,
        "alters_features",
        &anchors(TradeOp::Alters, &legacy.alters),
    );
}

fn write_format_2(out: &mut String, trades: &[Trade]) {
    if trades.is_empty() {
        return;
    }
    out.push_str("trades = [\n");
    for trade in trades {
        out.push_str("  { ");
        out.push_str(trade.op.slug());
        out.push_str(" = ");
        out.push_str(&tomlutil::string(&trade.printed));
        if let Some(feature) = &trade.feature {
            out.push_str(", feature = ");
            out.push_str(&tomlutil::string(&feature.anchor));
            if let Some(class) = &feature.class {
                out.push_str(", class = ");
                out.push_str(&tomlutil::id(class));
            }
            if !feature.levels.is_empty() {
                out.push_str(", levels = [");
                let levels: Vec<String> = feature
                    .levels
                    .iter()
                    .map(|level| level.to_string())
                    .collect();
                out.push_str(&levels.join(", "));
                out.push(']');
            }
            if let Some(power) = &feature.power {
                out.push_str(", power = ");
                out.push_str(&tomlutil::string(power));
            }
        }
        out.push_str(" },\n");
    }
    out.push_str("]\n");
}

/// True when `trades` use a key that corpus format 1 cannot write: a `class`, `levels` or
/// `power`, or an order that interleaves the two ops.
pub fn needs_format_2(trades: &[Trade]) -> bool {
    let narrowed = trades.iter().any(|trade| {
        trade.feature.as_ref().is_some_and(|feature| {
            feature.class.is_some() || !feature.levels.is_empty() || feature.power.is_some()
        })
    });
    let first_alter = trades.iter().position(|trade| trade.op == TradeOp::Alters);
    let interleaved = first_alter.is_some_and(|at| {
        trades[at..]
            .iter()
            .any(|trade| trade.op == TradeOp::Replaces)
    });
    narrowed || interleaved
}
