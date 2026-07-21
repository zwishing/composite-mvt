---
name: composition
description: "Skill for the Composition area of composite-mvt. 13 symbols across 6 files."
---

# Composition

13 symbols | 6 files | Cohesion: 40%

## When to Use

- Working with code in `tests/`
- Understanding how new, tile_with_layers, layer_names work
- Modifying composition-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `tests/composition/fixtures.rs` | layer_names, gunzip, brotli_encode, brotli_decode |
| `tests/composition/e2e.rs` | raw_two_layer_composer, emits_one_complete_gzip_output, emits_one_complete_zstd_output, emits_one_complete_brotli_output |
| `tests/composition/concurrency.rs` | arc_composer_is_safe_for_concurrent_requests, compressed_output_is_independent_across_threads |
| `src/source.rs` | new |
| `tests/common/mod.rs` | tile_with_layers |
| `tests/composition.rs` | compresses_the_complete_output_with_brotli_defaults |

## Entry Points

Start here when exploring this area:

- **`new`** (Function) — `src/source.rs:110`
- **`tile_with_layers`** (Function) — `tests/common/mod.rs:2`
- **`layer_names`** (Function) — `tests/composition/fixtures.rs:0`
- **`gunzip`** (Function) — `tests/composition/fixtures.rs:9`
- **`brotli_encode`** (Function) — `tests/composition/fixtures.rs:30`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `new` | Function | `src/source.rs` | 110 |
| `tile_with_layers` | Function | `tests/common/mod.rs` | 2 |
| `layer_names` | Function | `tests/composition/fixtures.rs` | 0 |
| `gunzip` | Function | `tests/composition/fixtures.rs` | 9 |
| `brotli_encode` | Function | `tests/composition/fixtures.rs` | 30 |
| `brotli_decode` | Function | `tests/composition/fixtures.rs` | 42 |
| `compresses_the_complete_output_with_brotli_defaults` | Function | `tests/composition.rs` | 182 |
| `arc_composer_is_safe_for_concurrent_requests` | Function | `tests/composition/concurrency.rs` | 11 |
| `compressed_output_is_independent_across_threads` | Function | `tests/composition/concurrency.rs` | 43 |
| `raw_two_layer_composer` | Function | `tests/composition/e2e.rs` | 36 |
| `emits_one_complete_gzip_output` | Function | `tests/composition/e2e.rs` | 47 |
| `emits_one_complete_zstd_output` | Function | `tests/composition/e2e.rs` | 67 |
| `emits_one_complete_brotli_output` | Function | `tests/composition/e2e.rs` | 87 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Compresses_the_complete_output_with_zstd_defaults → New` | cross_community | 6 |
| `Compresses_the_complete_output_with_brotli_defaults → New` | cross_community | 6 |
| `Mixed_input_encodings_preserve_all_layers → New` | cross_community | 6 |
| `Compresses_the_complete_output_with_brotli_defaults → Checked_total_from_lengths` | cross_community | 5 |
| `Rejects_wrong_input_count_before_composition → New` | cross_community | 5 |
| `Main → New` | cross_community | 4 |
| `Build → New` | cross_community | 4 |
| `Decompress → New` | cross_community | 4 |
| `Duplicate_source_ids_precede_per_source_errors → New` | cross_community | 4 |
| `Missing_layers_across_all_sources_precede_earlier_empty_names → New` | cross_community | 4 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Tests | 9 calls |

## How to Explore

1. `context({name: "new"})` — see callers and callees
2. `query({search_query: "composition"})` — find related execution flows
3. Read key files listed above for implementation details
4. `explain({target: "<file or symbol>"})` — persisted taint findings (source→sink data flows), when indexed with `--pdg`
