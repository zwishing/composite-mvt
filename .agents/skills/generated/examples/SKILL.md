---
name: examples
description: "Skill for the Examples area of composite-mvt. 4 symbols across 1 files."
---

# Examples

4 symbols | 1 files | Cohesion: 50%

## When to Use

- Working with code in `examples/`
- Understanding how main, tile_with_layers, gzip work
- Modifying examples-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `examples/mixed_sources.rs` | main, tile_with_layers, gzip, gunzip |

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `main` | Function | `examples/mixed_sources.rs` | 1 |
| `tile_with_layers` | Function | `examples/mixed_sources.rs` | 36 |
| `gzip` | Function | `examples/mixed_sources.rs` | 45 |
| `gunzip` | Function | `examples/mixed_sources.rs` | 54 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Main → New` | cross_community | 4 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Composition | 4 calls |
| Tests | 2 calls |

## How to Explore

1. `context({name: "main"})` — see callers and callees
2. `query({search_query: "examples"})` — find related execution flows
3. Read key files listed above for implementation details
4. `explain({target: "<file or symbol>"})` — persisted taint findings (source→sink data flows), when indexed with `--pdg`
