---
name: compression
description: "Skill for the Compression area of composite-mvt. 16 symbols across 2 files."
---

# Compression

16 symbols | 2 files | Cohesion: 68%

## When to Use

- Working with code in `src/`
- Understanding how decompress, compress work
- Modifying compression-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/compression/tests.rs` | none_decompression_borrows_input, other_compression_returns_unsupported_io_error, gzip_round_trip, zstd_round_trip, brotli_round_trip (+4) |
| `src/compression.rs` | decompress, decode_failure, decompress_gzip, decompress_zstd, decompress_brotli (+2) |

## Entry Points

Start here when exploring this area:

- **`decompress`** (Function) — `src/compression.rs:31`
- **`compress`** (Function) — `src/compression.rs:103`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `decompress` | Function | `src/compression.rs` | 31 |
| `compress` | Function | `src/compression.rs` | 103 |
| `decode_failure` | Function | `src/compression.rs` | 45 |
| `decompress_gzip` | Function | `src/compression.rs` | 56 |
| `decompress_zstd` | Function | `src/compression.rs` | 74 |
| `decompress_brotli` | Function | `src/compression.rs` | 86 |
| `none_decompression_borrows_input` | Function | `src/compression/tests.rs` | 30 |
| `compress_zstd` | Function | `src/compression.rs` | 134 |
| `other_compression_returns_unsupported_io_error` | Function | `src/compression/tests.rs` | 68 |
| `gzip_round_trip` | Function | `src/compression/tests.rs` | 131 |
| `zstd_round_trip` | Function | `src/compression/tests.rs` | 142 |
| `brotli_round_trip` | Function | `src/compression/tests.rs` | 153 |
| `disabled_gzip_compression_returns_unsupported_io_error` | Function | `src/compression/tests.rs` | 113 |
| `disabled_zstd_compression_returns_unsupported_io_error` | Function | `src/compression/tests.rs` | 119 |
| `disabled_brotli_compression_returns_unsupported_io_error` | Function | `src/compression/tests.rs` | 125 |
| `assert_unsupported` | Function | `src/compression/tests.rs` | 198 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Compresses_the_complete_output_with_zstd_defaults → New` | cross_community | 6 |
| `Compresses_the_complete_output_with_brotli_defaults → New` | cross_community | 6 |
| `Mixed_input_encodings_preserve_all_layers → New` | cross_community | 6 |
| `Rejects_wrong_input_count_before_composition → New` | cross_community | 5 |
| `Decompress → New` | cross_community | 4 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Composition | 5 calls |
| Tests | 2 calls |

## How to Explore

1. `context({name: "decompress"})` — see callers and callees
2. `query({search_query: "compression"})` — find related execution flows
3. Read key files listed above for implementation details
4. `explain({target: "<file or symbol>"})` — persisted taint findings (source→sink data flows), when indexed with `--pdg`
