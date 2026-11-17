//! The check that keeps the verify skill's feature map honest (`cargo xtask check`).
//!
//! AGENTS.md sends agents to the map to choose the e2e specs they run locally, so a spec or a
//! test that the map does not reach is skipped by everyone. The check reads the map, reads every
//! `*.spec.ts` under `tests/e2e/tests` (recursively), and fails when:
//!
//! - a spec file is not named in the map, or the map names a spec that does not exist;
//! - a `--grep` pattern is not a valid regular expression, or matches no test of one of its
//!   specs, or has a top-level alternative (`a|b`) that matches no test of any of its specs;
//! - a test is reached by no row for its spec: neither a "whole file" row nor a matching pattern;
//! - a row names specs without a pattern, or a pattern without specs.
//!
//! A pattern is matched the way Playwright matches `--grep`: a case-insensitive regular
//! expression (Playwright's `--grep` gives its pattern the `i` flag) tested against the file path, the titles of the enclosing `test.describe` blocks and
//! the test title, joined by spaces (Playwright also puts the project name in front, which the
//! check ignores). Test titles are read from the spec source without running a browser. A title
//! that is not a plain string (a template with `${}`, a variable, a concatenation) is reported as
//! unverifiable. It is never guessed.
//!
//! Map format. Each row of the table under the "Feature map" heading has the feature in the first
//! column and, in the second, one or more groups of a selector followed by the specs it applies
//! to in parentheses: `` `pattern` (`a.spec.ts`, `b.spec.ts`) `` or `whole file (`a.spec.ts`)`.
//! `\|` in a pattern is the table escape for `|`. Prose and `also` notes may follow inside the
//! parentheses. A row whose second column starts with `none` names no e2e spec.

use regex::{Regex, RegexBuilder};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

type Result<T = ()> = std::result::Result<T, String>;

/// The verify skill. Its feature map names the e2e spec that covers each feature.
pub const VERIFY_SKILL: &str = ".agents/skills/verify/SKILL.md";
/// The Playwright specs. Every `.spec.ts` file below this directory must be in the feature map.
pub const SPEC_DIR: &str = "tests/e2e/tests";

// --- The map -------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum Selector {
    /// Every test of the specs.
    Whole,
    /// A Playwright `--grep` pattern, with `\|` already turned into `|`.
    Pattern(String),
}

/// One selector of one row, with the specs it applies to.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Group {
    feature: String,
    line: usize,
    selector: Selector,
    specs: Vec<String>,
}

/// Spec paths named in `text`: runs of letters, digits and `._-/` that end in `.spec.ts`.
fn spec_names(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || "._-/".contains(c)))
        .filter(|word| word.len() > ".spec.ts".len() && word.ends_with(".spec.ts"))
        .map(str::to_string)
        .collect()
}

/// Splits a table row at every `|` that is not escaped as `\|`, and trims the cells.
fn split_row(line: &str) -> Vec<String> {
    let mut cells = vec![String::new()];
    let mut chars = line.trim().chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                cells.last_mut().unwrap().push_str("\\|");
                chars.next();
            }
            '|' => cells.push(String::new()),
            other => cells.last_mut().unwrap().push(other),
        }
    }
    // A row starts and ends with `|`, so the first and last pieces are empty.
    if cells.first().is_some_and(|c| c.trim().is_empty()) {
        cells.remove(0);
    }
    if cells.last().is_some_and(|c| c.trim().is_empty()) {
        cells.pop();
    }
    cells.into_iter().map(|c| c.trim().to_string()).collect()
}

fn is_separator_row(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells
            .iter()
            .all(|c| !c.is_empty() && c.chars().all(|ch| "-: ".contains(ch)))
}

/// The selector and specs of the second column.
fn parse_cell(cell: &str) -> Result<Vec<(Selector, Vec<String>)>> {
    let chars: Vec<char> = cell.chars().collect();
    let mut groups = Vec::new();
    let mut selector: Option<Selector> = None;
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '`' => {
                let close = chars[i + 1..]
                    .iter()
                    .position(|&c| c == '`')
                    .ok_or("a backtick is never closed")?;
                let code: String = chars[i + 1..i + 1 + close].iter().collect();
                selector = Some(Selector::Pattern(code.replace("\\|", "|")));
                i += close + 2;
            }
            '(' => {
                let mut depth = 0;
                let mut end = None;
                for (offset, &c) in chars[i..].iter().enumerate() {
                    match c {
                        '(' => depth += 1,
                        ')' => {
                            depth -= 1;
                            if depth == 0 {
                                end = Some(i + offset);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let end = end.ok_or("a parenthesis is never closed")?;
                let inner: String = chars[i + 1..end].iter().collect();
                let specs = spec_names(&inner);
                if !specs.is_empty() {
                    let Some(selector) = selector.take() else {
                        return Err(format!(
                            "names {} with no pattern before it; write `pattern` (spec) or whole file (spec)",
                            specs.join(", ")
                        ));
                    };
                    groups.push((selector, specs));
                }
                i = end + 1;
            }
            _ => {
                let rest: String = chars[i..].iter().take(10).collect();
                if rest.eq_ignore_ascii_case("whole file") {
                    selector = Some(Selector::Whole);
                    i += 10;
                } else {
                    i += 1;
                }
            }
        }
    }
    match selector {
        Some(Selector::Pattern(p)) => Err(format!("the pattern `{p}` is not followed by a spec")),
        Some(Selector::Whole) => Err("\"whole file\" is not followed by a spec".to_string()),
        None => Ok(groups),
    }
}

/// Every group of the feature map, and the problems found while reading it. The map is the table
/// rows under the heading that contains "Feature map", up to the next heading. Lines inside a
/// fenced code block are never headings or rows.
fn parse_map(skill: &str) -> Result<(Vec<Group>, Vec<String>)> {
    let mut in_map = false;
    let mut found_heading = false;
    let mut in_fence = false;
    let mut rows: Vec<(usize, Vec<String>)> = Vec::new();
    for (index, line) in skill.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
        } else if in_fence {
            continue;
        } else if line.starts_with('#') {
            in_map = line.contains("Feature map");
            found_heading |= in_map;
        } else if in_map && trimmed.starts_with('|') {
            rows.push((index + 1, split_row(line)));
        }
    }
    if !found_heading {
        return Err(format!("{VERIFY_SKILL} has no \"Feature map\" heading"));
    }
    let mut groups = Vec::new();
    let mut problems = Vec::new();
    for (position, (line, cells)) in rows.iter().enumerate() {
        let is_header = rows
            .get(position + 1)
            .is_some_and(|(_, next)| is_separator_row(next));
        if is_header || is_separator_row(cells) {
            continue;
        }
        let feature = cells.first().cloned().unwrap_or_default();
        let at = format!("{VERIFY_SKILL}:{line} (\"{feature}\")");
        let Some(cell) = cells.get(1) else {
            problems.push(format!("{at}: the row has no second column"));
            continue;
        };
        if cell.starts_with("none") {
            continue;
        }
        match parse_cell(cell) {
            Ok(found) if found.is_empty() => problems.push(format!(
                "{at}: names no spec; start the second column with `none:` if the feature has no e2e spec"
            )),
            Ok(found) => groups.extend(found.into_iter().map(|(selector, specs)| Group {
                feature: feature.clone(),
                line: *line,
                selector,
                specs,
            })),
            Err(message) => problems.push(format!("{at}: {message}")),
        }
    }
    Ok((groups, problems))
}

// --- The specs -----------------------------------------------------------------------------

/// A test found in a spec: the titles of its enclosing describe blocks, then its own.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TestTitle {
    path: Vec<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Scan {
    tests: Vec<TestTitle>,
    /// `(line, source text)` of each title that is not a plain string.
    unverifiable: Vec<(usize, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Prev {
    Start,
    /// A string, template, number or regular expression: a value, so a `/` after it divides.
    Value,
    Word(String),
    Punct(char),
    Arrow,
}

enum Arg {
    Literal(String),
    Computed(String),
}

enum Call {
    /// `test(...)`, `test.only(...)`: the first argument is the title.
    Test,
    /// `test.skip(...)` and kin: a title only when the first argument is a string.
    Modifier,
    /// `test.describe(...)` and variants.
    Describe,
    /// `test.use`, `test.step`, `test.describe.configure` and everything else.
    Ignore,
}

fn classify(segments: &[String]) -> Call {
    let names: Vec<&str> = segments.iter().map(String::as_str).collect();
    match names.as_slice() {
        [] | ["only"] => Call::Test,
        ["skip" | "fixme" | "fail" | "slow"] => Call::Modifier,
        ["describe", rest @ ..]
            if rest
                .iter()
                .all(|n| matches!(*n, "only" | "skip" | "fixme" | "serial" | "parallel")) =>
        {
            Call::Describe
        }
        _ => Call::Ignore,
    }
}

struct Scanner {
    /// The names a spec calls to declare tests: `test`, an import such as `installedTest`, and
    /// a name made by `.extend` from one of those.
    names: BTreeSet<String>,
    c: Vec<char>,
    i: usize,
    line: usize,
    depth: usize,
    prev: Prev,
    /// Describe blocks that are open: title (`None` when computed) and the brace depth of the body.
    scopes: Vec<(Option<String>, usize)>,
    /// A describe call whose body has not opened yet.
    pending: Option<Option<String>>,
    scan: Scan,
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '$'
}

fn is_ident_part(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$'
}

impl Scanner {
    fn at(&self, offset: usize) -> Option<char> {
        self.c.get(self.i + offset).copied()
    }

    fn advance(&mut self) {
        if self.at(0) == Some('\n') {
            self.line += 1;
        }
        self.i += 1;
    }

    fn skip_blank(&mut self) {
        while let Some(c) = self.at(0) {
            if c.is_whitespace() {
                self.advance();
            } else if c == '/' && self.at(1) == Some('/') {
                while self.at(0).is_some_and(|c| c != '\n') {
                    self.advance();
                }
            } else if c == '/' && self.at(1) == Some('*') {
                self.advance();
                self.advance();
                while self.at(0).is_some() && !(self.at(0) == Some('*') && self.at(1) == Some('/'))
                {
                    self.advance();
                }
                self.advance();
                self.advance();
            } else {
                break;
            }
        }
    }

    /// A quoted string, at its opening quote.
    fn read_string(&mut self) -> String {
        let quote = self.at(0).unwrap_or('"');
        self.advance();
        let mut out = String::new();
        while let Some(c) = self.at(0) {
            self.advance();
            match c {
                c if c == quote => break,
                '\\' => {
                    if let Some(next) = self.at(0) {
                        self.advance();
                        out.push(match next {
                            'n' => '\n',
                            't' => '\t',
                            other => other,
                        });
                    }
                }
                '\n' => break,
                other => out.push(other),
            }
        }
        out
    }

    /// A template literal, at its backtick: its text, or `None` when it has a `${}` part.
    fn read_template(&mut self) -> Option<String> {
        self.advance();
        let mut out = String::new();
        let mut plain = true;
        while let Some(c) = self.at(0) {
            if c == '`' {
                self.advance();
                break;
            }
            if c == '\\' {
                self.advance();
                if let Some(next) = self.at(0) {
                    out.push(next);
                    self.advance();
                }
            } else if c == '$' && self.at(1) == Some('{') {
                plain = false;
                self.advance();
                self.advance();
                self.skip_braces();
            } else {
                out.push(c);
                self.advance();
            }
        }
        plain.then_some(out)
    }

    /// The rest of a `${ ... }` part, after its opening brace.
    fn skip_braces(&mut self) {
        let mut depth = 1;
        while let Some(c) = self.at(0) {
            match c {
                '"' | '\'' => {
                    self.read_string();
                    continue;
                }
                '`' => {
                    self.read_template();
                    continue;
                }
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        self.advance();
                        return;
                    }
                }
                _ => {}
            }
            self.advance();
        }
    }

    /// A regular expression literal, at its opening slash.
    fn skip_regex(&mut self) {
        self.advance();
        let mut in_class = false;
        while let Some(c) = self.at(0) {
            match c {
                '\n' => return,
                '\\' => self.advance(),
                '[' => in_class = true,
                ']' => in_class = false,
                '/' if !in_class => {
                    self.advance();
                    break;
                }
                _ => {}
            }
            self.advance();
        }
        while self.at(0).is_some_and(is_ident_part) {
            self.advance();
        }
    }

    fn regex_allowed(&self) -> bool {
        match &self.prev {
            Prev::Start | Prev::Arrow => true,
            Prev::Value => false,
            Prev::Word(word) => matches!(
                word.as_str(),
                "return" | "typeof" | "case" | "in" | "of" | "await" | "yield" | "void" | "delete"
            ),
            Prev::Punct(c) => !matches!(c, ')' | ']' | '}'),
        }
    }

    /// The first argument of a call, after its `(`.
    fn first_argument(&mut self) -> Arg {
        self.skip_blank();
        let start = self.i;
        let literal = match self.at(0) {
            Some('"' | '\'') => Some(self.read_string()),
            Some('`') => self.read_template(),
            _ => None,
        };
        let started_with_quote = matches!(self.c.get(start), Some('"' | '\'' | '`'));
        if let Some(text) = literal {
            self.skip_blank();
            if matches!(self.at(0), Some(',' | ')')) {
                self.prev = Prev::Value;
                return Arg::Literal(text);
            }
        }
        let snippet: String = self.c[start..]
            .iter()
            .take_while(|&&c| c != '\n')
            .take(60)
            .collect();
        if started_with_quote {
            self.prev = Prev::Value;
        }
        Arg::Computed(snippet.trim_end().to_string())
    }

    fn call(&mut self, segments: &[String]) {
        let line = self.line;
        let kind = classify(segments);
        if matches!(kind, Call::Ignore) {
            return;
        }
        let arg = self.first_argument();
        match (kind, arg) {
            (Call::Test, Arg::Literal(title)) | (Call::Modifier, Arg::Literal(title)) => {
                self.add_test(title, line);
            }
            (Call::Test, Arg::Computed(text)) => self.scan.unverifiable.push((line, text)),
            (Call::Modifier, Arg::Computed(_)) => {}
            (Call::Describe, Arg::Literal(title)) => self.pending = Some(Some(title)),
            (Call::Describe, Arg::Computed(text)) => {
                self.scan.unverifiable.push((line, text));
                self.pending = Some(None);
            }
            (Call::Ignore, _) => {}
        }
    }

    fn add_test(&mut self, title: String, line: usize) {
        let mut path = Vec::new();
        for (scope, _) in &self.scopes {
            match scope {
                Some(text) => path.push(text.clone()),
                None => {
                    self.scan.unverifiable.push((
                        line,
                        format!("{title:?} sits in a describe block with a computed title"),
                    ));
                    return;
                }
            }
        }
        path.push(title);
        self.scan.tests.push(TestTitle { path });
    }

    fn run(&mut self) {
        loop {
            self.skip_blank();
            let Some(c) = self.at(0) else { return };
            match c {
                '"' | '\'' => {
                    self.read_string();
                    self.prev = Prev::Value;
                }
                '`' => {
                    self.read_template();
                    self.prev = Prev::Value;
                }
                '/' => {
                    if self.regex_allowed() {
                        self.skip_regex();
                        self.prev = Prev::Value;
                    } else {
                        self.advance();
                        self.prev = Prev::Punct('/');
                    }
                }
                '{' => {
                    self.depth += 1;
                    if matches!(self.prev, Prev::Arrow | Prev::Punct(')'))
                        && let Some(title) = self.pending.take()
                    {
                        self.scopes.push((title, self.depth));
                    }
                    self.advance();
                    self.prev = Prev::Punct('{');
                }
                '}' => {
                    if self
                        .scopes
                        .last()
                        .is_some_and(|(_, depth)| *depth == self.depth)
                    {
                        self.scopes.pop();
                    }
                    self.depth = self.depth.saturating_sub(1);
                    self.advance();
                    self.prev = Prev::Punct('}');
                }
                '>' => {
                    let arrow = self.prev == Prev::Punct('=');
                    self.advance();
                    self.prev = if arrow { Prev::Arrow } else { Prev::Punct('>') };
                }
                c if is_ident_start(c) => self.word(),
                c if c.is_ascii_digit() => {
                    while self.at(0).is_some_and(|c| is_ident_part(c) || c == '.') {
                        self.advance();
                    }
                    self.prev = Prev::Value;
                }
                c => {
                    self.advance();
                    self.prev = Prev::Punct(c);
                }
            }
        }
    }

    fn read_ident(&mut self) -> String {
        let mut word = String::new();
        while let Some(c) = self.at(0).filter(|&c| is_ident_part(c)) {
            word.push(c);
            self.advance();
        }
        word
    }

    fn word(&mut self) {
        let after_dot = self.prev == Prev::Punct('.');
        let word = self.read_ident();
        if !self.names.contains(&word) || after_dot {
            self.prev = Prev::Word(word);
            return;
        }
        let mut segments = Vec::new();
        while self.at(0) == Some('.') && self.at(1).is_some_and(is_ident_start) {
            self.advance();
            segments.push(self.read_ident());
        }
        self.skip_blank();
        if self.at(0) == Some('(') {
            self.advance();
            self.prev = Prev::Punct('(');
            self.call(&segments);
        } else {
            self.prev = Prev::Word(word);
        }
    }
}

/// The names a spec source uses to declare tests. `test` always counts. So does a name imported
/// from `./fixtures` or `@playwright/test` whose exported name ends in `test` (in any case), under
/// its local alias, and a local `const x = <declarer>.extend(...)`.
fn declarer_names(source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::from(["test".to_string()]);
    let import = Regex::new(
        r#"import\s*(?:type\s*)?\{([^}]*)\}\s*from\s*["'](?:\./fixtures|@playwright/test)["']"#,
    )
    .expect("a fixed pattern");
    for clause in import.captures_iter(source) {
        for item in clause[1].split(',') {
            let mut parts = item.split_whitespace();
            let (Some(imported), rest) = (parts.next(), parts.collect::<Vec<_>>()) else {
                continue;
            };
            let local = match rest.as_slice() {
                ["as", local] => local,
                _ => imported,
            };
            if imported.to_ascii_lowercase().ends_with("test") {
                names.insert(local.to_string());
            }
        }
    }
    let extend =
        Regex::new(r"(?:const|let)\s+(\w+)\s*=\s*(\w+)\.extend\b").expect("a fixed pattern");
    loop {
        let before = names.len();
        for found in extend.captures_iter(source) {
            if names.contains(&found[2]) {
                names.insert(found[1].to_string());
            }
        }
        if names.len() == before {
            break;
        }
    }
    names
}

/// The tests of a spec source, and its titles that cannot be read statically.
fn scan_spec(source: &str) -> Scan {
    let mut scanner = Scanner {
        names: declarer_names(source),
        c: source.chars().collect(),
        i: 0,
        line: 1,
        depth: 0,
        prev: Prev::Start,
        scopes: Vec::new(),
        pending: None,
        scan: Scan::default(),
    };
    scanner.run();
    scanner.scan
}

/// The top-level alternatives of a regular expression: the parts between `|` that is outside
/// parentheses, brackets and escapes. An empty part stays, since it matches every title.
fn split_alternatives(pattern: &str) -> Vec<String> {
    let mut parts = vec![String::new()];
    let (mut depth, mut in_class, mut escaped) = (0usize, false, false);
    for c in pattern.chars() {
        let split = !escaped && !in_class && depth == 0 && c == '|';
        if split {
            parts.push(String::new());
            continue;
        }
        if escaped {
            escaped = false;
        } else {
            match c {
                '\\' => escaped = true,
                '[' if !in_class => in_class = true,
                ']' if in_class => in_class = false,
                '(' if !in_class => depth += 1,
                ')' if !in_class => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        parts.last_mut().unwrap().push(c);
    }
    parts
}

// --- The comparison -----------------------------------------------------------------------

/// What Playwright greps for a test: the file path, the describe titles and the test title.
fn grep_text(file: &str, test: &TestTitle) -> String {
    format!("{file} {}", test.path.join(" "))
}

#[derive(Debug, Default)]
pub struct Report {
    /// What must change. The check fails when this is not empty.
    pub problems: Vec<String>,
    /// Titles the check cannot read. They are reported, not failed.
    pub unverifiable: Vec<String>,
    /// The number of spec files read.
    pub specs: usize,
    /// The number of tests matched by a row.
    pub tests: usize,
}

/// Compares the map in `skill` with `specs` (path below [`SPEC_DIR`] to source text).
fn analyse(skill: &str, specs: &BTreeMap<String, String>) -> Result<Report> {
    let (groups, mut problems) = parse_map(skill)?;
    let mut report = Report {
        specs: specs.len(),
        ..Report::default()
    };
    let scans: BTreeMap<&str, Scan> = specs
        .iter()
        .map(|(path, source)| (path.as_str(), scan_spec(source)))
        .collect();

    let mapped: BTreeSet<&str> = groups
        .iter()
        .flat_map(|g| g.specs.iter().map(String::as_str))
        .collect();
    for path in scans.keys() {
        if !mapped.contains(path) {
            problems.push(format!(
                "{SPEC_DIR}/{path} is not in the feature map; add it to a row of {VERIFY_SKILL}"
            ));
        }
    }
    for name in &mapped {
        if !scans.contains_key(name) {
            problems.push(format!(
                "the feature map names {name}, but {SPEC_DIR}/{name} does not exist; fix or remove the row in {VERIFY_SKILL}"
            ));
        }
    }

    let mut covered: BTreeMap<&str, Vec<bool>> = scans
        .iter()
        .map(|(path, scan)| (*path, vec![false; scan.tests.len()]))
        .collect();
    for group in &groups {
        let at = format!("{VERIFY_SKILL}:{} (\"{}\")", group.line, group.feature);
        let existing: Vec<&str> = group
            .specs
            .iter()
            .map(String::as_str)
            .filter(|name| scans.contains_key(name))
            .collect();
        let matcher = match &group.selector {
            Selector::Whole => None,
            Selector::Pattern(pattern) => {
                match RegexBuilder::new(pattern).case_insensitive(true).build() {
                    Ok(regex) => Some(regex),
                    Err(error) => {
                        problems.push(format!(
                            "{at}: the pattern `{pattern}` is not a regular expression this check can read: {}",
                            error.to_string().lines().last().unwrap_or("invalid")
                        ));
                        continue;
                    }
                }
            }
        };
        for name in &existing {
            let mut hits = 0;
            for (index, test) in scans[name].tests.iter().enumerate() {
                if matcher
                    .as_ref()
                    .is_none_or(|regex| regex.is_match(&grep_text(name, test)))
                {
                    covered.get_mut(name).unwrap()[index] = true;
                    hits += 1;
                }
            }
            if let Selector::Pattern(pattern) = &group.selector
                && hits == 0
            {
                problems.push(format!(
                    "{at}: the pattern `{pattern}` matches no test of {name}"
                ));
            }
        }
        if let Selector::Pattern(pattern) = &group.selector {
            let alternatives = split_alternatives(pattern);
            if alternatives.len() > 1 {
                for alternative in alternatives {
                    let regex = RegexBuilder::new(&alternative)
                        .case_insensitive(true)
                        .build();
                    let reached = regex.as_ref().is_ok_and(|regex| {
                        existing.iter().any(|name| {
                            scans[name]
                                .tests
                                .iter()
                                .any(|test| regex.is_match(&grep_text(name, test)))
                        })
                    });
                    if !reached && !existing.is_empty() {
                        problems.push(format!(
                            "{at}: the alternative `{alternative}` of the pattern `{pattern}` matches no test of {}; remove it",
                            existing.join(", ")
                        ));
                    }
                }
            }
        }
    }
    for (path, flags) in covered.iter().filter(|(path, _)| mapped.contains(*path)) {
        for (test, hit) in scans[path].tests.iter().zip(flags) {
            if *hit {
                report.tests += 1;
            } else {
                problems.push(format!(
                    "{SPEC_DIR}/{path}: \"{}\" is reached by no row of the feature map; widen a pattern or add a row (or use whole file) in {VERIFY_SKILL}",
                    test.path.join(" › ")
                ));
            }
        }
    }
    for (path, scan) in &scans {
        for (line, text) in &scan.unverifiable {
            report.unverifiable.push(format!(
                "{SPEC_DIR}/{path}:{line}: the title is not a plain string, so no row can be checked against it: {text}"
            ));
        }
    }
    report.problems = problems;
    Ok(report)
}

// --- The files ----------------------------------------------------------------------------

/// Every `*.spec.ts` below `dir`, by path relative to `dir` with `/` separators.
fn read_specs(dir: &Path) -> Result<BTreeMap<String, String>> {
    fn walk(base: &Path, dir: &Path, found: &mut BTreeMap<String, String>) -> Result {
        let listing =
            fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
        let mut paths: Vec<PathBuf> = Vec::new();
        for entry in listing {
            paths.push(
                entry
                    .map_err(|e| format!("cannot read {}: {e}", dir.display()))?
                    .path(),
            );
        }
        paths.sort();
        for path in paths {
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name != "node_modules") {
                    walk(base, &path, found)?;
                }
            } else if path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().ends_with(".spec.ts"))
            {
                let relative = path
                    .strip_prefix(base)
                    .map_err(|e| e.to_string())?
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                let source = fs::read_to_string(&path)
                    .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
                found.insert(relative, source);
            }
        }
        Ok(())
    }
    let mut found = BTreeMap::new();
    walk(dir, dir, &mut found)?;
    Ok(found)
}

/// Fails when the feature map and the spec files in the repository at `root` disagree.
pub fn check(root: &Path) -> Result {
    let skill_path = root.join(VERIFY_SKILL);
    let skill = fs::read_to_string(&skill_path)
        .map_err(|e| format!("cannot read {}: {e}", skill_path.display()))?;
    let report = analyse(&skill, &read_specs(&root.join(SPEC_DIR))?)?;
    for note in &report.unverifiable {
        println!("check-feature-map: unverifiable: {note}");
    }
    if report.problems.is_empty() {
        println!(
            "check-feature-map: all {} specs of {SPEC_DIR} are in the feature map, and its rows reach all {} tests",
            report.specs, report.tests
        );
        Ok(())
    } else {
        Err(format!(
            "check-feature-map: the feature map and {SPEC_DIR} disagree:\n  {}",
            report.problems.join("\n  ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC_A: &str = r#"
import { test } from "./fixtures";
// test("a comment is not a test", async () => {});
const slash = 4 / 2; const re = /test\("x"/g; const text = 'test("in a string")';
test("alpha one", async ({ page }) => {
  test.skip(page.browserName === "webkit", "not on webkit");
  await test.step("a step is not a test", async () => {});
});
test.skip("alpha skipped", async () => {});
test.describe("Group", () => {
  test("beta", async () => {});
  test.describe.serial("Inner", () => {
    test("gamma", async () => {});
  });
  test("delta", async () => {});
});
test.describe("Tagged", { tag: "@x" }, () => {
  test("epsilon", async () => {});
});
test("after", async () => {});
"#;

    fn titles(scan: &Scan) -> Vec<String> {
        scan.tests.iter().map(|t| t.path.join(" › ")).collect()
    }

    #[test]
    fn the_scanner_reads_titles_and_describe_nesting() {
        let scan = scan_spec(SPEC_A);
        assert_eq!(
            titles(&scan),
            [
                "alpha one",
                "alpha skipped",
                "Group › beta",
                "Group › Inner › gamma",
                "Group › delta",
                "Tagged › epsilon",
                "after",
            ]
        );
        assert!(scan.unverifiable.is_empty(), "{:?}", scan.unverifiable);
    }

    #[test]
    fn a_title_that_is_not_a_plain_string_is_reported_not_guessed() {
        let source = r#"
const name = "x";
test(`plain ${name}`, async () => {});
test(name, async () => {});
test("one " + name, async () => {});
test(`no substitution`, async () => {});
for (const k of ["a"]) {
  test(`${k} loop`, async () => {});
}
test.describe(`Dynamic ${name}`, () => {
  test("hidden", async () => {});
});
test("fine", async () => {});
"#;
        let scan = scan_spec(source);
        assert_eq!(titles(&scan), ["no substitution", "fine"]);
        let lines: Vec<usize> = scan.unverifiable.iter().map(|(line, _)| *line).collect();
        assert_eq!(lines, [3, 4, 5, 8, 10, 11], "{:?}", scan.unverifiable);
    }

    #[test]
    fn imported_and_extended_declarers_are_read() {
        let source = r#"
import { test as base, installedTest, expect } from "./fixtures";
import { test as pw } from "@playwright/test";
const mine = installedTest.extend<{}>({});
installedTest("one", async () => {});
mine.describe("G", () => { mine("two", async () => {}); });
pw("three", async () => {});
base("four", async () => {});
other("five", async () => {});
"#;
        // `test as base` imports the name `test`, so it is a declarer under its local name.
        assert_eq!(
            titles(&scan_spec(source)),
            ["one", "G › two", "three", "four"]
        );
    }

    #[test]
    fn a_template_title_is_read_when_it_has_no_substitution() {
        let scan = scan_spec("test(`a ${\"}\"} b`, () => {});\ntest(`plain`, () => {});");
        assert_eq!(titles(&scan), ["plain"]);
        assert_eq!(scan.unverifiable.len(), 1);
    }

    #[test]
    fn rows_split_at_unescaped_pipes_only() {
        assert_eq!(
            split_row("| Feature | `a\\|b` (`x.spec.ts`) | `src.rs` |"),
            ["Feature", "`a\\|b` (`x.spec.ts`)", "`src.rs`"]
        );
        assert!(is_separator_row(&split_row("|---|:--:|---|")));
    }

    #[test]
    fn a_cell_has_selectors_followed_by_specs() {
        let cell =
            "`a\\|b` (`x.spec.ts`, y.spec.ts; also `mobile-chromium`), whole file (`z.spec.ts`)";
        assert_eq!(
            parse_cell(cell).unwrap(),
            [
                (
                    Selector::Pattern("a|b".into()),
                    vec!["x.spec.ts".into(), "y.spec.ts".into()]
                ),
                (Selector::Whole, vec!["z.spec.ts".into()]),
            ]
        );
        assert!(
            parse_cell("`only a pattern`")
                .unwrap_err()
                .contains("not followed")
        );
        assert!(
            parse_cell("(`x.spec.ts`)")
                .unwrap_err()
                .contains("no pattern")
        );
        assert!(parse_cell("whole file").is_err());
        // A parenthetical without a spec is prose.
        assert_eq!(parse_cell("`p` (note) (`x.spec.ts`)").unwrap().len(), 1);
    }

    fn map(rows: &str) -> String {
        format!(
            "# Verify\n\n```sh\n# Feature map (a comment in a fence)\n```\n\n## 5. Feature map\n\n| Feature | grep | src |\n|---|---|---|\n{rows}\n## Next\n\n| Not | in `late.spec.ts` | map |\n"
        )
    }

    fn sources() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("a.spec.ts".to_string(), SPEC_A.to_string()),
            (
                "sub/b.spec.ts".to_string(),
                "test.describe(\"Sub\", () => { test(\"one\", () => {}); test(\"two\", () => {}); });".to_string(),
            ),
        ])
    }

    const FULL: &str = "| One | whole file (`a.spec.ts`) | `x.rs` |\n| Two | `one` (`sub/b.spec.ts`) | `y.rs` |\n| Three | `two\\|Sub` (`sub/b.spec.ts`) | `y.rs` |\n| Docs | none: `python3 x.py` | `x.py` |\n";

    #[test]
    fn a_complete_map_passes() {
        let report = analyse(&map(FULL), &sources()).unwrap();
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert_eq!(report.specs, 2);
        assert_eq!(report.tests, 9);
    }

    #[test]
    fn a_pattern_is_matched_like_playwright_against_path_describe_and_title() {
        // The file path and a describe title count, case does not.
        let rows = "| One | `A.SPEC` (`a.spec.ts`) | `x` |\n| Two | `^sub/b.spec.ts sub one` (`sub/b.spec.ts`) | `y` |\n| Three | `sub two` (`sub/b.spec.ts`) | `y` |\n";
        let report = analyse(&map(rows), &sources()).unwrap();
        assert!(report.problems.is_empty(), "{:?}", report.problems);
    }

    #[test]
    fn a_missing_spec_a_stale_name_and_a_dead_pattern_are_listed() {
        let rows = "| One | whole file (`a.spec.ts`, `gone.spec.ts`) | `x` |\n| Two | `nothing like it` (`sub/b.spec.ts`) | `y` |\n";
        let problems = analyse(&map(rows), &sources()).unwrap().problems;
        let all = problems.join("\n");
        assert!(
            all.contains("gone.spec.ts, but") || all.contains("names gone.spec.ts"),
            "{all}"
        );
        assert!(all.contains("matches no test of sub/b.spec.ts"), "{all}");
        // The spec's tests are not reached either, since the only row is dead.
        assert!(all.contains("\"Sub › one\" is reached by no row"), "{all}");
        let missing = analyse(
            &map("| One | whole file (`a.spec.ts`) | `x` |\n"),
            &sources(),
        )
        .unwrap()
        .problems;
        // The tests of a spec the map lacks are not listed one by one.
        assert_eq!(missing.len(), 1, "{missing:?}");
        assert!(
            missing[0].contains("sub/b.spec.ts is not in the feature map"),
            "{missing:?}"
        );
    }

    #[test]
    fn a_pattern_must_match_in_every_spec_it_lists_and_every_alternative_must_match() {
        // `two` matches in sub/b.spec.ts only, so it is dead for a.spec.ts.
        let rows = "| One | whole file (`a.spec.ts`) | `x` |\n| Two | whole file (`sub/b.spec.ts`) | `y` |\n| Three | `two` (`a.spec.ts`, `sub/b.spec.ts`) | `z` |\n";
        let problems = analyse(&map(rows), &sources()).unwrap().problems;
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(
            problems[0].contains("matches no test of a.spec.ts"),
            "{problems:?}"
        );
        // One dead alternative among live ones.
        let rows = "| One | whole file (`a.spec.ts`) | `x` |\n| Two | whole file (`sub/b.spec.ts`) | `y` |\n| Three | `two\\|beta\\|no such title` (`a.spec.ts`, `sub/b.spec.ts`) | `z` |\n";
        let problems = analyse(&map(rows), &sources()).unwrap().problems;
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(
            problems[0].contains("alternative `no such title`")
                && problems[0].contains("remove it"),
            "{problems:?}"
        );
    }

    #[test]
    fn alternatives_split_only_at_top_level_pipes() {
        assert_eq!(split_alternatives("a|b"), ["a", "b"]);
        assert_eq!(
            split_alternatives("(a|b)c|[|]d|e\\|f"),
            ["(a|b)c", "[|]d", "e\\|f"]
        );
        assert_eq!(split_alternatives("a|"), ["a", ""]);
        assert_eq!(split_alternatives("plain"), ["plain"]);
    }

    #[test]
    fn a_test_that_no_row_reaches_is_listed() {
        let rows =
            "| One | `alpha` (`a.spec.ts`) | `x` |\n| Two | whole file (`sub/b.spec.ts`) | `y` |\n";
        let problems = analyse(&map(rows), &sources()).unwrap().problems;
        assert!(
            problems.iter().any(|p| p.contains("\"Group › beta\"")),
            "{problems:?}"
        );
        assert!(
            !problems.iter().any(|p| p.contains("alpha one")),
            "{problems:?}"
        );
    }

    #[test]
    fn row_shape_problems_and_bad_patterns_are_listed() {
        let rows = "| One | `x` | `x` |\n| Two | prose only | `y` |\n| Three | `(unclosed` (`a.spec.ts`) | `y` |\n";
        let problems = analyse(&map(rows), &sources()).unwrap().problems.join("\n");
        assert!(problems.contains("not followed by a spec"), "{problems}");
        assert!(problems.contains("names no spec"), "{problems}");
        assert!(problems.contains("not a regular expression"), "{problems}");
    }

    #[test]
    fn unverifiable_titles_are_reported_without_failing() {
        let mut files = sources();
        files.insert("c.spec.ts".into(), "test(`x ${1}`, () => {});".into());
        let rows = format!("{FULL}| Four | whole file (`c.spec.ts`) | `z` |\n");
        let report = analyse(&map(&rows), &files).unwrap();
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert_eq!(report.unverifiable.len(), 1);
        assert!(
            report.unverifiable[0].contains("c.spec.ts:1"),
            "{:?}",
            report.unverifiable
        );
    }

    #[test]
    fn a_hash_in_a_fenced_block_is_not_a_heading() {
        // The fence holds "# Feature map" before the real heading, and a later fence holds a
        // "# heading" that must not end the map.
        let skill = "# T\n\n## Feature map\n\n| F | g | s |\n|---|---|---|\n| One | whole file (`a.spec.ts`) | x |\n\n```sh\n# not a heading\n| fake | whole file (`ghost.spec.ts`) | x |\n```\n| Two | whole file (`sub/b.spec.ts`) | x |\n";
        let problems = analyse(skill, &sources()).unwrap().problems;
        assert!(problems.is_empty(), "{problems:?}");
        assert!(analyse("# T\n```\n## Feature map\n```\n", &sources()).is_err());
    }

    #[test]
    fn specs_are_found_recursively() {
        let dir =
            std::env::temp_dir().join(format!("cast-iron-xtask-specs-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("deep/er")).unwrap();
        fs::create_dir_all(dir.join("node_modules/pkg")).unwrap();
        fs::write(dir.join("top.spec.ts"), "test(\"t\", () => {});").unwrap();
        fs::write(dir.join("deep/er/low.spec.ts"), "test(\"l\", () => {});").unwrap();
        fs::write(dir.join("helpers.ts"), "export {};").unwrap();
        fs::write(dir.join("node_modules/pkg/skip.spec.ts"), "").unwrap();
        let found: Vec<String> = read_specs(&dir).unwrap().into_keys().collect();
        assert_eq!(found, ["deep/er/low.spec.ts", "top.spec.ts"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_committed_feature_map_matches_the_spec_files() {
        check(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    }
}
