# 0005. Expanding the dictionary to 100k words: ECDICT import + faster suggestion lookup

- **Status**: accepted
- **Date**: 2026-06-10

## Context

Until this ADR, the active dictionary was `word_en_v3.sqlite` = 20,000 words (the top
20k by frequency). Users reported that mid-frequency words were missing (e.g.
`indicator` / `gist`) and asked to expand to **100,000 words**.

Investigation found:

1. The original generator `Core/src/bin/generator.rs` was **stale** — it read a
   `Dict/raw_50k.txt` that no longer exists in the repo and emitted `[mock meaning]`
   placeholder definitions for almost every word; the shipped v1–v3 were not produced
   by it and cannot be reproduced from it.
2. The real data source is **ECDICT** (its MIT license already ships in
   `Dict/ECDICT-LICENSE.txt`). The full set is ~3.4M rows, but **only ~57k words carry
   any frequency / exam tag** (`frq>0` or `bnc>0`).
3. The suggestion path `ee_core::rank_candidates` ran a full Levenshtein scan over the
   entire in-memory word list on every keystroke, plus one `to_lowercase()` heap
   allocation per candidate. Fine at 20k, laggy at 100k.

## Decision

### Data
- Use the ECDICT prebuilt SQLite (`stardict` table) as the import source, placed at
  `Dict/ecdict.db` and **not committed** (~800 MB build input, added to `.gitignore`).
- Rewrite `generator.rs` to read it via rusqlite (already a Core dependency — no new
  dependency) and emit a **new `word_en_v4.sqlite` + `word_list_v4`**, leaving v1–v3
  untouched. The app's highest-version `word_en_v{N}` / `word_list_v{N}` scan picks v4
  automatically, so the platform layer needs no change.
- **Selection** (top 100k): require a Chinese `translation`; drop proper nouns
  (capitalised original) and pure inflected forms (ECDICT `exchange` `0:lemma` points to
  another word, whose lemma carries the inflections); rank by commonness — contemporary
  frequency `frq` → BNC `bnc` → Collins/Oxford/exam tag → remaining real words that have
  a phonetic — and keep the top 100k.
- Field mapping: `phonetic`→`ipa`, `translation` (per line)→`major`+`definitions`,
  `exchange`→`inflections` (`p/d/i/3/s`), no examples/audio.

### Suggestion speedup (`Core/algo`, results identical to the original, no bucketing)
- Allocation-free ASCII case-insensitive prefix check.
- Exact length-band pre-filter (skip the DP for non-prefix candidates whose length gap
  exceeds 3, using "edit distance ≥ length difference").
- Bounded, early-exiting Levenshtein (stop once a row minimum exceeds 3, which is sound
  because the per-row minimum is non-decreasing).

## Consequences

- **Positive**
  - 100k coverage; mid-frequency words such as `indicator`/`gist` are now present;
    definitions / phonetics / inflections all come from real corpus data.
  - Suggestions run at ~4.6 ms/keystroke over 100k words (measured), so typing stays
    smooth; a new `optimized_matches_naive_reference` test proves the optimized scan
    returns the same results as a naive implementation.
  - No new dependency; zero platform-layer changes (version scan auto-selects v4).
- **Trade-offs**
  - Repo grows by ~28 MB (`word_en_v4.sqlite` 27 MB + `word_list_v4` 0.8 MB; v1–v3 are
    already committed as precedent).
  - The tail of the 100k (~58k words) is "real but obscure" because ECDICT only has ~57k
    words with a frequency signal; proper nouns and inflected forms are filtered out, but
    technical/archaic words remain. To ship only high-quality common words, lower
    `TARGET_WORDS` to ~57k.
  - The ECDICT source DB (`Dict/ecdict.db`) must be downloaded from an ECDICT release
    before the dataset can be regenerated.
- **Known pre-existing issue (unrelated, not fixed)**
  - `Core/tests/test_hub.rs::hub_concurrently_queries_three_real_dbs` asserts that
    `apply`'s IPA is identical across v1/v2/v3, but v3 is ECDICT-style (`ə'plai`) while
    v1/v2 are the old style (`əˈplaɪ`); this test already failed before this change. This
    task keeps v1–v3 untouched, so it is not fixed here.

## Update (2026-06-10): consolidated to a single dictionary

The legacy `word_en_v1/v2/v3` (5k/10k/20k) datasets were removed; the 100k ECDICT
dataset originally emitted as v4 was renamed to **v1** and is now the only bundled
dictionary. The paired headword list is `Dict/word_en_v1` (no extension), sharing the
`word_en_v1` base name with `Dict/word_en_v1.sqlite` and distinguished from it by the
`.sqlite` suffix. The app's word-list discovery (`App/Win/src/dict.rs`) was updated to
match `word_en_v{N}` files without the `.sqlite` extension, and the generator now writes
these two files. The two multi-tier integration tests were rewritten to query the single
dictionary, which also retires the pre-existing IPA-mismatch failure noted above (the
old v1/v2 data it depended on no longer exists).

## Update (2026-06-11): broadened coverage (~206k headwords)

The blanket "drop every capitalised original" rule excluded common proper nouns —
countries, place names, languages, months — so basic words like `china` / `france`
were missing. The selection now admits a capitalised word when it carries a corpus
frequency signal (`frq`/`bnc`), which includes the common proper nouns while still
excluding the obscure long tail (surnames, hamlets). The `TARGET_WORDS` cap was raised
to 300k so the quality gate — not an arbitrary cap — bounds the set; the dataset now has
~205.8k English headwords (and the inverted Chinese set ~85.2k terms, which likewise now
maps to capitalised English equivalents). Keys remain lower-cased, so lookups stay
case-insensitive.

## References

- ECDICT (data source + field / exchange format): https://github.com/skywind3000/ECDICT
- Selection and mapping implementation: `Core/src/bin/generator.rs`
- Suggestion speedup and equivalence test: `Core/src/algo/mod.rs`
