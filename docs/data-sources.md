# Data Sources

This project embeds the generated dictionary into `dict.exe` at build time. The
runtime binary does not download or read external dictionary files.

## Current Source

- Repository: `https://github.com/KyleBing/english-vocabulary.git`
- Pinned ref: `8814e02b40f69a2a6e016dbde087010304fcedfc`
- Source directory: `json/`
- Lock file: `data/sources.lock`

The release workflow checks out the pinned ref from `data/sources.lock` and
generates `data/generated/dictionary.json` before compiling the binary.

## Imported Files

- `1-初中-顺序.json`, imported as `COMMON_3500`
- `2-高中-顺序.json`, imported without a display tag
- `3-CET4-顺序.json`, imported as `CET4`
- `4-CET6-顺序.json`, imported as `CET6`
- `5-考研-顺序.json`, imported without a display tag
- `6-托福-顺序.json`, imported without a display tag
- `7-SAT-顺序.json`, imported without a display tag

## Release Audit

Every release writes `dist/dataset-audit.txt` with:

- source repository
- source commit actually checked out
- generated entry count
- tagged and untagged entry counts
- tag distribution

If lookup quality regresses, start with that audit file. It tells you which data
input produced the binary.

## Licensing Status

The source is externally maintained. Before broad redistribution, verify the
upstream license and keep any required attribution in this repository and release
artifacts.
