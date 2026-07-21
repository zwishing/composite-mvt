---
name: tests
description: "Skill for the Tests area of composite-mvt. 50 symbols across 10 files."
---

# Tests

50 symbols | 10 files | Cohesion: 64%

## When to Use

- Working with code in `tests/`
- Understanding how builder, with_compression, compose work
- Modifying tests-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `tests/source_parsing.rs` | parses_uncompressed_layers_in_order, rejects_a_nonempty_tile_without_layers, maps_an_explicit_empty_layer_name_to_missing_layer_name, preserves_duplicate_layer_names_from_one_sample_for_builder_validation, automatically_parses_gzip_sample (+7) |
| `tests/builder_validation.rs` | source, rejects_no_sources_and_duplicate_ids, explicit_duplicate_validation_matches_build, allows_cross_source_duplicates_when_configured, same_source_duplicates_are_always_invalid (+5) |
| `tests/composition.rs` | decompresses_each_source_before_composing, composes_all_layers_from_concatenated_gzip_members, reports_the_source_that_failed_decompression, compresses_the_complete_output_with_dependency_defaults, compresses_the_complete_output_with_zstd_defaults (+4) |
| `src/source.rs` | with_compression, from_mvt, from_mvt_with_compression, from_mvts_with_compression, decompress (+2) |
| `src/composer.rs` | builder, checked_total_len, checked_total_from_lengths, compose, compose_raw |
| `src/compression.rs` | compress_gzip, compress_brotli, detect_compression |
| `tests/composition/e2e.rs` | mixed_input_encodings_preserve_all_layers |
| `tests/composition/fixtures.rs` | zstd_encode |
| `src/builder.rs` | default |
| `tests/common/mod.rs` | gzip |

## Entry Points

Start here when exploring this area:

- **`builder`** (Function) — `src/composer.rs:33`
- **`with_compression`** (Function) — `src/source.rs:123`
- **`compose`** (Function) — `src/composer.rs:64`
- **`zstd_encode`** (Function) — `tests/composition/fixtures.rs:20`
- **`from_mvt`** (Function) — `src/source.rs:149`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `builder` | Function | `src/composer.rs` | 33 |
| `with_compression` | Function | `src/source.rs` | 123 |
| `compose` | Function | `src/composer.rs` | 64 |
| `zstd_encode` | Function | `tests/composition/fixtures.rs` | 20 |
| `from_mvt` | Function | `src/source.rs` | 149 |
| `from_mvt_with_compression` | Function | `src/source.rs` | 164 |
| `from_mvts_with_compression` | Function | `src/source.rs` | 215 |
| `decompress` | Function | `src/source.rs` | 263 |
| `gzip` | Function | `tests/common/mod.rs` | 11 |
| `detect_compression` | Function | `src/compression.rs` | 7 |
| `from_mvts` | Function | `src/source.rs` | 189 |
| `source` | Function | `tests/builder_validation.rs` | 2 |
| `rejects_no_sources_and_duplicate_ids` | Function | `tests/builder_validation.rs` | 7 |
| `explicit_duplicate_validation_matches_build` | Function | `tests/builder_validation.rs` | 25 |
| `allows_cross_source_duplicates_when_configured` | Function | `tests/builder_validation.rs` | 46 |
| `same_source_duplicates_are_always_invalid` | Function | `tests/builder_validation.rs` | 75 |
| `duplicate_source_ids_precede_per_source_errors` | Function | `tests/builder_validation.rs` | 95 |
| `missing_layers_across_all_sources_precede_earlier_empty_names` | Function | `tests/builder_validation.rs` | 112 |
| `duplicate_layers_precede_unsupported_source_compression` | Function | `tests/builder_validation.rs` | 129 |
| `duplicate_layers_precede_disabled_source_compression` | Function | `tests/builder_validation.rs` | 154 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Compresses_the_complete_output_with_zstd_defaults → New` | cross_community | 6 |
| `Compresses_the_complete_output_with_brotli_defaults → New` | cross_community | 6 |
| `Mixed_input_encodings_preserve_all_layers → New` | cross_community | 6 |
| `Compresses_the_complete_output_with_zstd_defaults → Checked_total_from_lengths` | intra_community | 5 |
| `Compresses_the_complete_output_with_brotli_defaults → Checked_total_from_lengths` | cross_community | 5 |
| `Mixed_input_encodings_preserve_all_layers → Checked_total_from_lengths` | intra_community | 5 |
| `Rejects_wrong_input_count_before_composition → Checked_total_from_lengths` | cross_community | 5 |
| `Rejects_wrong_input_count_before_composition → New` | cross_community | 5 |
| `Main → New` | cross_community | 4 |
| `Duplicate_source_ids_precede_per_source_errors → New` | cross_community | 4 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Composition | 18 calls |
| Compression | 1 calls |

## How to Explore

1. `context({name: "builder"})` — see callers and callees
2. `query({search_query: "tests"})` — find related execution flows
3. Read key files listed above for implementation details
4. `explain({target: "<file or symbol>"})` — persisted taint findings (source→sink data flows), when indexed with `--pdg`
