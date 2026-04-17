# Offline Dict CLI Design

## Summary

Build a fast offline English-Chinese CLI dictionary for Windows.

The tool is intentionally narrow:

- Query single English words or fixed phrases
- Query simplified Chinese words or fixed phrases
- English lookup is case-insensitive exact match
- Chinese lookup is simplified-Chinese exact match
- No fuzzy search, prefix search, stemming, pinyin, examples, or online features
- Ship as a single `dict.exe`
- Embed one compact built-in dictionary dataset directly into the executable

Primary goal: replace the "powerful but awkward" experience of generic offline dictionary tools with a fast, deterministic, exam-tag-aware CLI focused on daily word lookup.

## Goals

- Fast startup on Windows
- Single executable distribution
- Deterministic query behavior
- Clean simplified-Chinese output
- Show exam/common-word tags for English entries
- Keep implementation and maintenance scope small

## Non-Goals

- Sentence translation
- Interactive REPL
- User-custom dictionary imports
- Runtime database initialization
- Runtime external dictionary files
- Auto-suggestions
- Morphological normalization such as `apples -> apple`
- Traditional-Chinese normalization
- Pronunciation, audio, pinyin, examples

## User-Facing Behavior

### Commands

```text
dict <query>
dict --all <query>
dict --help
dict --version
```

### Query Rules

- If the input contains Chinese characters, perform simplified-Chinese exact match
- If the input consists of English word/phrase characters, perform case-insensitive exact match on the English headword
- Do not perform fuzzy matching
- Do not perform stemming or lemmatization
- Do not split phrases into component words

Examples:

```text
dict apple
dict Apple
dict apple pie
dict 苹果
dict 放弃
```

### Default Output

For English lookup:

```text
abandon
tags: CET4 CET6 TEM4 TEM8 GRE
1. 放弃
2. 遗弃
3. 沉湎于
```

For simplified-Chinese lookup:

```text
放弃
1. abandon
2. give up
3. renounce
4. quit
5. relinquish

5 of 12 results, use --all to show more
```

### Output Rules

- Default output shows at most 5 results
- `--all` shows all exact-match results
- English query prints tags once at the headword level
- Chinese query does not print tags by default
- No spelling suggestions are shown on misses

### Error Handling

- No result: print `未找到精确匹配: <query>` and return a non-zero exit code
- Empty input: print short help
- Invalid flags: standard CLI error output

## Runtime Architecture

The final distributed artifact is one executable:

- `dict.exe`

There is no runtime `dict.db`, no same-directory dictionary file lookup, and no startup-time data processing.

Two phases exist conceptually:

- Build-time data pipeline
- Runtime query engine

At runtime, the executable:

1. Parses CLI arguments
2. Determines whether the query is Chinese or English
3. Looks up the built-in read-only dictionary data
4. Formats results
5. Exits

This keeps startup fast and distribution simple.

## Data Model

### Dictionary Entry

Each logical English entry contains:

- `english_headword`
- `english_headword_norm`
- `simplified_chinese_defs[]`
- `tag_bits`

### Query Indexes

Two read-only lookup paths are required:

- English normalized headword -> English entry
- Simplified Chinese string -> list of English headwords

Because the total working set is around 12,000 entries, the implementation does not require SQLite in v1.

## Tag System

The tool should display these tags when an English word is queried:

- `COMMON_3500`
- `CET4`
- `CET6`
- `TEM4`
- `TEM8`
- `GRE`

### Tag Semantics

- Tags apply only to English headwords
- Tag matching is exact at the normalized English headword level
- Tags do not propagate through stemming, phrase splitting, or partial containment

Examples:

- `abandon` matches `abandon`
- `abandoned` does not inherit `abandon`
- `apple pie` does not inherit `apple`

### Tag Storage

Store tags as bitflags on the English entry, for example a `u32` or `u64`.

This makes runtime tag display constant-time and compact.

## Sorting Rules

For Chinese-to-English lookup, rank exact-match English candidates using this priority:

1. `COMMON_3500`
2. `CET4`
3. `CET6`
4. `TEM4`
5. `TEM8`
6. `GRE`
7. Stable alphabetical order

This ensures common and exam-relevant words appear before rarer words.

For English lookup, the tool returns the unique matched English headword block and lists simplified-Chinese definitions in source-preserved order after deduplication.

## Data Sources

### Dictionary Meanings

Use an open English-Chinese dictionary source suitable for offline redistribution and simplified-Chinese output.

Current preferred direction:

- Use a source derived from or compatible with CC-CEDICT-style open data
- Normalize output to simplified Chinese only

### Tag Sources

Keep tag sources separate from dictionary meanings.

Suggested build-time files:

- `data/tags/common_3500.txt`
- `data/tags/cet4.txt`
- `data/tags/cet6.txt`
- `data/tags/tem4.txt`
- `data/tags/tem8.txt`
- `data/tags/gre.txt`

This separation allows independent replacement of:

- meaning sources
- exam/common-word tag sets

### Source Confidence and Licensing Notes

- `CET4` and `CET6` have the clearest official basis
- `TEM4` and `TEM8` are feasible but may rely on cleaned public lists when official machine-readable lists are unavailable
- `GRE` requires the most care due to licensing ambiguity in many public lists
- `COMMON_3500` should be defined from a clean, redistributable list rather than a vague "popular list"

Before shipping publicly, each source must be checked for redistribution compatibility.

## Build-Time Pipeline

The runtime binary should not do any heavy lifting. All normalization happens before compilation.

Suggested build-time workflow:

1. Load raw dictionary source
2. Normalize English headwords
   - lowercase
   - trim repeated whitespace
3. Normalize Chinese definitions
   - simplified Chinese only
   - remove display noise
4. Load tag files
5. Normalize tag words using the same English normalization rule
6. Merge tag bitflags into dictionary entries
7. Build compact read-only lookup data
8. Embed generated data into the Rust binary at compile time

## Repository Layout

Suggested project root layout:

```text
offline-dict-cli/
  Cargo.toml
  src/
  data/
    dictionary/
    tags/
  tools/
  tests/
```

Within `data/tags/`, store one source file per tag set.

Within `tools/`, store the build-time normalization and generation scripts.

## Testing Strategy

### Runtime Tests

Cover:

- English exact lookup
- English case-insensitive behavior
- Chinese exact lookup
- Default top-5 truncation
- `--all` expansion
- Tag rendering correctness
- Ranking correctness for Chinese-to-English output
- Miss behavior and non-zero exit code
- Help behavior on empty input

### Build-Time Validation

Validate:

- No empty headwords
- No empty simplified-Chinese definition sets
- Tag source files contain valid normalized words
- Duplicate tag source lines are tolerated or cleaned
- English normalization collisions are handled deterministically

## Implementation Choice

Preferred implementation stack:

- Rust for the runtime CLI
- Build-time scripts for dictionary and tag normalization
- Embedded static data for the final executable

Why Rust:

- Small and fast executable
- Good Windows CLI distribution story
- Predictable startup behavior
- Enough control for compact read-only embedded data structures

## Scope Guardrails for v1

v1 must remain intentionally small. Do not add:

- fuzzy search
- prefix search
- stemming
- suggestions
- pinyin
- examples
- audio
- online fallback
- plugin architecture
- user dictionaries
- runtime hot reload of data

## Open Follow-Up Items

These are follow-up tasks, not open design ambiguity:

- Choose final redistributable meaning source
- Choose final redistributable tag source for each tag family
- Decide exact binary name and project folder name
- Decide whether `COMMON_3500` should be internally displayed as `COMMON_3500` or a shorter user-facing label such as `COMMON`
