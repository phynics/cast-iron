---
title: "A book membership without a citation row is an error, with a corpus allow-list"
date: 2026-11-25
status: current
superseded_by: []
issues: [461]
prs: []
area: corpus
---

# A book membership without a citation row is an error, with a corpus allow-list, 2026-11-25

Issue 461 cleaned the corpus of book memberships that no `[[sources]]` row supports: an entry
lists a book in `source_ids` (other than its first) and nothing cites it. The warning that
measured the cleanup fell from 1,471 memberships in 355 entries to 1 with the pin of corpus
v0.1.51 (data pull request 316). The owner ruled on 2026-11-25 that the rule becomes an error
once the pin carries that cleanup, with one named exception: Gamemastery in the GameMastery Guide,
which waits for the home-book question (phynics/cast-iron-data#277, after v1).

## The rule

`corpus-check` reports each uncited membership as an error, one per entry and book, with the path
and the line of `source_ids`. `cargo xtask content` runs the same check, so it fails too. The
rule is the one of [`docs/corpus-format.md`](../corpus-format.md): the first book needs no row; a
later book needs a `[[sources]]` row (with or without a page); a body `**Sources:**` line and an
`expanded_in` credit do not count.

## The exception list lives in the corpus

The exception is data about the corpus, so it lives beside it, not in the app: a file
`membership-exceptions.toml` in the corpus root, optional and read only by `corpus-check`. It
follows `power-exceptions.toml` ([the power exceptions entry](2026-11-21-class-option-power-exceptions.md)): `[[exception]]`
rows, `deny_unknown_fields`, a `reason` and an `issue` on each row.

```toml
[[exception]]
entry = "ref/rule/gamemastery"
book = "game-mastery-guide"
reason = "…"
issue = "phynics/cast-iron-data#277"
```

The list only shrinks. A row is an error when it excepts nothing: its entry is gone, the entry no
longer lists the book, the book is the entry's first, or a `[[sources]]` row cites the book now.
That makes resolving the membership a two-step change that the check enforces: add the row (or
remove the book), then remove the exception, in the same corpus change. The file is not a way to
silence a new finding; a new uncited membership is a cleanup item. An absent file is an empty
list, so the app needs no exception of its own.

The alternatives were an app-side list (the app would carry corpus facts, and the pin and the list
would drift apart) and a deny-by-count threshold (it would let a new membership pass as long as
another left). Neither keeps the list tied to the entry it names.

## Order of the change

The pinned corpus (v0.1.51) has no such file. With the rule an error and no file, `cargo xtask
content` fails on the one Gamemastery membership. So the app change merges only after a data tag
that adds `membership-exceptions.toml`; the pin of `sources/corpus.toml` moves to that tag before this change merges (or in it). No format
version changes: the file is not part of the compiled library, and the entries are unchanged.
