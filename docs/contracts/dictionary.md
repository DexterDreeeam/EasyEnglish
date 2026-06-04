# `dictionary` Contract

**Source path**: `src/core/dictionary/`
**Owner test path**: `tests/unit/dictionary/`
**Status**: lookup + suggest frozen (iter-002 + iter-006)

## 1. Public API (FROZEN — change requires ADR)

```cpp
namespace easyenglish::core::dictionary {

enum class DictError {
    NotFound,
    InvalidInput,
    StorageError,
};

struct Entry {
    std::string headword;
    std::string phonetic;       // IPA, may be empty
    std::vector<std::string> definitions;  // one per sense; non-empty when present
};

class IDictionary {
public:
    virtual ~IDictionary() = default;

    // Exact lookup. Empty input → DictError::InvalidInput.
    virtual auto lookup(std::string_view word) const
        -> std::expected<Entry, DictError> = 0;

    // Suggestions, ordered by ascending edit distance (ties broken by frequency
    // if available, otherwise lexicographically). `max` caps the result length.
    // Added in iter-006 — implementations may stub until then.
    virtual auto suggest(std::string_view prefix, std::size_t max = 10) const
        -> std::vector<std::string> = 0;
};

}  // namespace easyenglish::core::dictionary
```

## 2. Invariants

- `lookup` is thread-safe for concurrent `const` calls on the same instance.
- `lookup` is case-insensitive; the returned `Entry::headword` preserves the
  dictionary's canonical casing (not the caller's input).
- `suggest("")` returns an empty vector — never `DictError`.
- `suggest` is deterministic for a given dictionary state.

## 3. Error codes

| Code | Meaning |
|---|---|
| `NotFound`     | Word not present in dictionary |
| `InvalidInput` | Empty string, or input longer than 128 chars |
| `StorageError` | Wrap of `storage::StorageError` — see `cause()` (TBD) |

## 4. Dependencies

- Allowed: `src/core/storage`, `nlohmann_json::nlohmann_json`, `src/core/network` (for `ApiDictionary`)
- Forbidden: Qt, ImGui, GLFW, any UI library, file system outside what storage exposes

## 5. Test fixtures

- `tests/fixtures/mini_dict.sqlite`
- `tests/fixtures/fuzzy/<prefix>.golden` — JSON arrays of expected suggestions,
  one file per probe.

## 6. Performance budget

- `lookup` p99 < **500us** on the mini fixture (iter-002 to verify and adjust).
- `suggest` p99 < **5ms** on a 100k-entry corpus (iter-006 to verify).

## 7. Change log

- 2026-06-04 — iter-002: `lookup` implemented + frozen. `SqliteDictionary` over
  the `entries` table from iter-001 fixture, prepared statement cached, mutex
  serializes shared access. `suggest()` stubbed to empty pending iter-006.
- 2026-06-04 — iter-006: `suggest()` implemented + frozen. Brute-force
  Levenshtein over an in-memory cache of all headwords loaded at `open()`.
  Ordered by ascending edit distance, alphabetical tiebreak (stable sort on
  the pre-sorted cache). Golden tests under `tests/fixtures/fuzzy/`.
  Caveat: for ≥ 100k-entry corpora the brute force is O(n) per call; an
  index (prefix trie + edit-distance pruning) is a future iteration.
- 2026-06-04 — iter-007: second `IDictionary` implementation `ApiDictionary`
  added. Backed by injected `network::INetworkClient` (default endpoint:
  dictionaryapi.dev). Tests use a hand-rolled `MockNetworkClient` — no real
  HTTP traffic in CI.
- 2026-06-04 — iter-009: switched JSON parser from `QJsonDocument` to
  `nlohmann/json`. `ApiDictionary` API also moved from `QString` to
  `std::string`. No behavioral change — invariants unchanged. See ADR-0002.
- 2026-06-04 — iter-011: **data semantics changed from EN→EN to EN→CN.**
  The `Entry::definitions` vector now carries Chinese translations.
  Public interface and error codes are unchanged. `ApiDictionary` endpoint
  switched from dictionaryapi.dev to MyMemory (`api.mymemory.translated.net`)
  and parses `responseData.translatedText` + `matches[].translation`.
  See ADR-0003.
