# 0006. Chinese → English dictionary (`word_cn`) + two-level preview focus

- **Status**: accepted
- **Date**: 2026-06-10

## Context

The app shipped only an English → Chinese dictionary (`word_en`). Users also want
to type Chinese and discover the English word(s) for it. Requirements:

- A **separate** record provider for Chinese → English.
- One Chinese term maps to **up to 10 English words**, ordered by English usage frequency.
- Typing Chinese lists **up to 5 preview rows**; each row shows the Chinese term on the
  left and the **top-3 English words** as buttons on the right.
- Keyboard interaction: Up/Down pick the Chinese row; Right enters the row's English
  buttons; Left/Right move between buttons (Left from the first returns to the row);
  Up/Down on a button leave the row; Enter/Space/click on a button runs an exact lookup
  of that English word and switches to the English Card view.

## Decision

### Data — invert the existing ECDICT
- No new data source: `Core/src/bin/generator.rs` reuses `Dict/ecdict.db` and, alongside
  the English dataset, builds an inverted Chinese → English index.
- For each clean, frequency-ranked English base word, split its Chinese `translation`
  into terms — strip POS prefixes and bracketed domain/clarification tags, split on
  CJK/ASCII separators, keep pure-CJK pieces of length 1..=8.
- For each Chinese term, collect the English words mapping to it, rank by English usage
  frequency (`frq`, then BNC-only; words with no frequency signal are excluded so the
  ordering is meaningful), dedup, and keep the **top 10**.
- Emit `Dict/word_cn_v1.sqlite` (`storage_entries`, value = serialized
  `RecordModel::WordCn`) + `Dict/word_cn_v1` (sorted Chinese headword list, no extension).
  Result: ~75.5k Chinese terms (~8.4 MB).

### Core — new record type (interface change)
- Add `WordCn { word: String, english: Vec<String> }` and `RecordModel::WordCn`
  (serde tag `word_cn`); export from `lib.rs`; document in `Core/.interface.md`.
- The Core interface was frozen at iter-014; this additive variant is recorded here.

### App
- `App/Win/src/dict.rs`: discovery is parameterized by base prefix, so `word_cn_v{N}.sqlite`
  (DB) and `word_cn_v{N}` (list, distinguished by the missing `.sqlite`) are found the
  same way as `word_en`.
- `overlay.rs`: the Chinese Storage is added as a **separate provider** on the same Hub
  (Latin keys only hit the English DB, Chinese keys only hit the Chinese DB). Input with
  any CJK char selects Chinese mode; up to 5 query keys come from `rank_candidates` over
  the Chinese list. Rendering branches on the deserialized record type.
- Two-level focus is a pure function `cn_focus_step` (row vs. button), unit-tested, with
  Up/Down on rows, Right/Left on buttons. Activating a button reuses the existing
  `! word` jump-to-exact path (ADR 0005 / section 1.8 of the App UX spec).

## Consequences

- **Positive**
  - Chinese → English search with no new dependency or data source; English flow unchanged.
  - Equivalents are ordered by real corpus frequency; previews stay compact (top-3 of 10).
  - Discovery, hub, and the exact-lookup jump are reused, keeping the change localized.
- **Trade-offs**
  - Repo grows by ~9 MB (`word_cn_v1.sqlite` 8.4 MB + `word_cn_v1` 0.8 MB).
  - Inverted terms inherit ECDICT noise (e.g. occasional non-English glosses like `pomme`);
    acceptable for a lookup aid. Tightening would need an English-only post-filter.
  - The record stores 10 English words but the UI only surfaces the top 3; the rest are
    currently unused beyond storage (room for a future "more" affordance).
  - The 360px overlay height cap can clip the bottom "Search on Bing" row when 5 Chinese
    rows are shown; a scroll/auto-grow layout is deferred.

## Update (2026-06-11): Chinese matching is exact + prefix only

Chinese term matching was switched from fuzzy (prefix + edit distance) to **exact +
prefix only** via a new `ee_core::prefix_candidates`. Edit-distance suggestions were
noisy for Chinese, where a one-character difference is usually a different word. The
overlay's Chinese branch now uses `prefix_candidates` (exact term first, then closest
prefixes); the English flow keeps fuzzy `rank_candidates`.

## References

- ECDICT (source corpus): https://github.com/skywind3000/ECDICT
- Inversion + term extraction: `Core/src/bin/generator.rs`
- Record type: `Core/src/record/word_cn.rs`
- Mode detection, rendering, and two-level focus: `App/Win/src/overlay.rs`
- UX spec: `App/.design.md` section 1.9
