# Implementation Plan

## Goals

Build a safe and fast Rust crate for UST parsing:

- no `unsafe` in the parser;
- zero-copy AST over `&[u8]` for structural parsing;
- explicit decoding helpers for Shift_JIS and declared charsets;
- lossless preservation of unknown fields and sections;
- typed accessors for common settings and note fields;
- test coverage for classic UST, plugin UST, charset handling, and malformed input.

## Current Scope

Implemented:

- byte-level parser for sections and `key=value` lines;
- known header classification: `VERSION`, `SETTING`, `TRACKEND`, numbered notes, `PREV`, `NEXT`, `INSERT`, `DELETE`;
- common typed note fields;
- `Charset=` detection from the first ten lines;
- Shift_JIS default decoding through `encoding_rs`;
- tests for normal UST 1.2, plugin headers, charset detection, Shift_JIS decoding, and basic structural errors.

## Next Implementation Steps

1. Add a writer that round-trips through the byte AST.
2. Add a high-level owned model for callers that want decoded strings and normalized timing.
3. Implement UST 2-style timing position calculation from `Delta` and `Duration`.
4. Parse pitch, vibrato, and envelope lists into typed structures while retaining raw bytes.
5. Add corpus tests using real-world UST files from permissively licensed sources.
6. Add fuzzing with `cargo fuzz` or `arbitrary` to harden malformed byte input.
7. Add benchmarks for large UST files and compare byte parsing against decoded string parsing.

## API Direction

Keep two layers:

- `parse_bytes`: low-level, zero-copy, lossless AST. This should remain the core fast path.
- future `Project`: owned, decoded, semantically normalized model for applications.

The low-level layer should not reject unknown keys or uncommon extensions. UST has no single complete official specification, and practical compatibility depends on tolerating fields emitted by UTAU, plugins, OpenUtau, Utsu, LibreSVIP, and other tools.
