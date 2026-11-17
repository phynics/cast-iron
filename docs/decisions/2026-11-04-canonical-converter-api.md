---
title: "Canonical converter API"
date: 2026-11-04
status: current
superseded_by: []
issues: []
prs: []
area: corpus
---

# Canonical converter API, 2026-11-04

- **One converter, two selections.** `canonical::convert_corpus` is the production-grade conversion of the screened in-scope Pf Data inventory. `canonical::convert_representative` runs the same core over the bounded fixture selection. The `data-report` and `data-import` commands are now reporting layers: schema validation, search checks, fixture evidence, and report files stay in `pf_data`, but conversion exists once. Nothing calls the core from `cargo xtask content`, so production remains the PSRD build.
- **Conversion and the alias catalog share one id derivation.** `pf_derived::native_id` delegates to `alias::canonical_native_id`, so every native entry is `ref/<route-family>/<key>` and no second implementation can drift. Derived racial traits, afflictions, and allowlisted rules already use their `ref/` forms; parent links and link targets are the same ids. A full-corpus test asserts every converted id is offered by the alias catalog and every catalog id is either converted or listed in `excluded` with a reason.
- **A failed record is excluded and reported.** `Converted.excluded` names the upstream identity, the canonical id when one was derived, the category (`native`, `derived`, or `rule`), and the reason. The existing finding keeps the converter defect; the exclusion states that no entry was emitted. Neither a failed record nor a rejected derivation fails the run, so rule 6 is satisfied without turning the corpus into a build failure. A duplicate id is still a rejection, never a suffix.
- **Synthesized parent-to-child links use `build::NestedLink`.** A derived racial trait or affliction is `child: false` (a typed record the host nests); a promoted rule is `child: true` (a child of the host rules tree). Their `trail` is empty, like the reviewed-supplement links, because this layer inserts the link itself rather than resolving an upstream heading at render time. The field is left for the later book-order and relations work.
- **Reports changed only where the id namespace did.** `data-report` still reproduces its committed files byte-for-byte, but the 222 derived records in `reports/migration-coverage.json` now name their parent as `ref/...` instead of `pf-data/...`, and the representative sample's serialized byte counts shrink (`1179224` content, `208006` search). The full import keeps 29,253 selected and 27,853 converted records, the same 6/6 fixtures, and all findings except one search check that left the first-100 window when `ref/` ids reordered the corpus. `cargo xtask content` remains byte-identical to `origin/main` (`content_version` `vdb43853600b8ed8b`, 9,968 entries).
