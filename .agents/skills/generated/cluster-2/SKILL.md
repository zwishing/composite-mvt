---
name: cluster-2
description: "Skill for the Cluster_2 area of composite-mvt. 6 symbols across 2 files."
---

# Cluster_2

6 symbols | 2 files | Cohesion: 86%

## When to Use

- Working with code in `src/`
- Understanding how validate_duplicate_layers, build, feature_enabled work
- Modifying cluster_2-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/builder.rs` | validate_duplicate_layers, build, validate, validate_source_compression, validate_output_compression |
| `src/compression.rs` | feature_enabled |

## Entry Points

Start here when exploring this area:

- **`validate_duplicate_layers`** (Function) — `src/builder.rs:61`
- **`build`** (Function) — `src/builder.rs:96`
- **`feature_enabled`** (Function) — `src/compression.rs:21`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `validate_duplicate_layers` | Function | `src/builder.rs` | 61 |
| `build` | Function | `src/builder.rs` | 96 |
| `feature_enabled` | Function | `src/compression.rs` | 21 |
| `validate` | Function | `src/builder.rs` | 105 |
| `validate_source_compression` | Function | `src/builder.rs` | 149 |
| `validate_output_compression` | Function | `src/builder.rs` | 167 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Build → New` | cross_community | 4 |
| `Build → Compression` | intra_community | 4 |
| `Build → Id` | intra_community | 4 |
| `Build → Feature_enabled` | intra_community | 4 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Composition | 2 calls |

## How to Explore

1. `context({name: "validate_duplicate_layers"})` — see callers and callees
2. `query({search_query: "cluster_2"})` — find related execution flows
3. Read key files listed above for implementation details
4. `explain({target: "<file or symbol>"})` — persisted taint findings (source→sink data flows), when indexed with `--pdg`
