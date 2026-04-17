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
dict search-log
dict search-log --from 2026-04-10 --to 2026-04-17
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
tags: CET4
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
- English query prints exactly one display tag, the lowest tag in the configured tag chain
- Chinese query does not print tags by default
- No spelling suggestions are shown on misses

### CLI Parsing Rules

- `search-log` is interpreted as a subcommand when it is the first positional token
- `--all` only applies to lookup queries
- `dict search-log --all` is an error
- `dict --all search-log` is treated as a lookup query for the literal text `search-log`
- `dict search-log` does not accept extra positional arguments
- Invalid command combinations return standard CLI argument errors

### Error Handling

- No result: print `未找到精确匹配: <query>` and return a non-zero exit code
- Empty input: print short help
- Invalid flags: standard CLI error output

## Query History

The tool should automatically keep a lightweight local history of successful
English lookups.

This feature is intentionally narrow:

- Only successful English exact-match lookups are recorded
- Chinese reverse lookups are not recorded
- Misses are not recorded
- History is local-only and per-user
- History is grouped by day
- Within one day, each normalized English headword appears at most once

### History Commands

```text
dict search-log
dict search-log --from 2026-04-10 --to 2026-04-17
```

### History Query Rules

- `dict search-log` defaults to the most recent 7 natural days, including today
- `--from` and `--to` must be provided together
- Date format is `YYYY-MM-DD`
- The date range is inclusive on both ends
- Output is ordered from newest day to oldest day
- Within each day, words are shown in first-query order
- Runtime date uses the local system date
- Tests must use an injected clock seam rather than wall-clock time

### History Recording Rules

- A successful English lookup records the normalized English headword
- Normalization uses the same lowercase and whitespace-normalization rule as English lookup
- If the same normalized headword is queried again on the same day, do not write it again
- English phrase lookups follow the same rule if the phrase exists as an English entry
- Because the distributed dataset is currently words-only, v1 history will effectively contain words rather than phrases
- In normal single-process use, history preserves first-query order within a day
- Under concurrent multi-process writes, history is best effort; strict first-query ordering and perfect on-disk de-duplication are not guaranteed
- History reads must de-duplicate repeated lines while preserving the first appearance in the file

### History Output

Example default output:

```text
2026-04-18
1. abandon
2. apple

2026-04-17
(no queries)

2026-04-16
1. persist
```

Output rules:

- Always print each day in the requested range, even if there were no queries
- Empty days print exactly `(no queries)`
- `dict search-log` returns a success exit code even if the whole range is empty

### History Storage

Store history outside the executable directory in the current user's local app
data directory.

On Windows, use:

```text
%LOCALAPPDATA%\offline-dict-cli\log\
```

Allow an explicit override:

```text
OFFLINE_DICT_HISTORY_DIR
```

`OFFLINE_DICT_HISTORY_DIR` points directly to the directory that contains
daily files such as `2026-04-18.txt`. It is not a higher-level root and must
not have `log\` appended automatically.

Store one plain-text file per day:

```text
2026-04-18.txt
2026-04-17.txt
```

Each file contains one normalized English headword per line.

### History Error Handling

- `dict search-log --from <date>` without `--to` is an error
- `dict search-log --to <date>` without `--from` is an error
- Invalid date text is an error
- `from > to` is an error
- Failure to write history must not break a successful dictionary lookup; print a short warning to stderr and still return success for the lookup itself
- A missing history directory is treated as empty history
- A missing day file is treated as `(no queries)` for that day
- Empty or malformed lines inside a readable day file are ignored
- If `OFFLINE_DICT_HISTORY_DIR` points to a file instead of a directory, `dict search-log` returns an error
- If the resolved history directory exists but is unreadable, `dict search-log` returns an error
- If `%LOCALAPPDATA%` is unavailable and `OFFLINE_DICT_HISTORY_DIR` is not set, `dict search-log` returns an error

## Runtime Architecture

The final distributed artifact is one executable:

- `dict.exe`

There is no runtime `dict.db`, no same-directory dictionary file lookup, and no startup-time data processing.

Two phases exist conceptually:

- Build-time data pipeline
- Runtime query engine

At runtime, the executable:

1. Parses CLI arguments
2. Either executes a history query or determines whether the lookup query is Chinese or English
3. Looks up the built-in read-only dictionary data when needed
4. Records the successful English lookup in local history when applicable
5. Formats results
6. Exits

This keeps startup fast and distribution simple.

### Runtime Command Model

The runtime should parse arguments into an explicit command enum rather than
handling behavior through ad hoc positional parsing.

Suggested shape:

- `Lookup { query, show_all }`
- `SearchLog { from, to }`
- `Help`
- `Version`

`SearchLog` must not load or parse the embedded dictionary dataset.

Runtime execution should flow through a testable command runner that accepts:

- resolved runtime paths and environment-derived configuration
- a clock abstraction for current local date
- the dictionary only for lookup commands

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
- The display priority chain is `COMMON_3500 < CET4 < CET6 < TEM4 < TEM8 < GRE`
- English lookup displays only the lowest tag in that chain
- `COMMON_3500` is a valid display tag and is the lowest display level
- Runtime data must retain the full tag set for ranking and internal logic
- User-facing display must use a separate derived display-tag path rather than overloading the full tag collection

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
- Automatic recording of successful English lookups
- No history write for Chinese lookups
- No history write for misses
- `dict search-log` default 7-day range behavior
- `dict search-log --from ... --to ...` range behavior
- Day-local de-duplication
- Invalid date and invalid history-flag error cases
- Missing-history-directory behavior
- Invalid-history-directory behavior
- History read tolerance for empty or malformed lines
- Successful lookup with history-write warning behavior
- Lowest-tag display behavior for English lookup
- Chinese ranking remains based on the full tag set rather than the single display tag
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
- A small date library such as `chrono` for date parsing and day-range handling

Why Rust:

- Small and fast executable
- Good Windows CLI distribution story
- Predictable startup behavior
- Enough control for compact read-only embedded data structures

### Runtime Implementation Notes

- Centralize runtime environment parsing for dataset path and history path overrides
- Prefer incremental day-file writes over full-file rewrites
- A history write should read the current day file only as needed to decide whether an append is necessary

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
- cross-device sync for history
- query-count statistics
- per-query timestamps in user-facing output

## Open Follow-Up Items

These are follow-up tasks, not open design ambiguity:

- Choose final redistributable meaning source
- Choose final redistributable tag source for each tag family
- Decide exact binary name and project folder name
- Decide whether `COMMON_3500` should be internally displayed as `COMMON_3500` or a shorter user-facing label such as `COMMON`
