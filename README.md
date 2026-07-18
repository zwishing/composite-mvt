# composite-mvt

`composite-mvt` composes a fixed set of Mapbox Vector Tile (MVT) sources into one response body.
It is deliberately a byte-level compositor: it does not parse, merge, rename, or validate request
tile layers at compose time. Configure the source metadata once, then supply one byte slice per
source for every request.

## Quick start

```rust
use composite_mvt::{Compression, DuplicateLayer, MvtComposer, MvtSource};

# fn run(roads: &[u8], buildings_gzip: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
let composer = MvtComposer::builder()
    .duplicate_layer(DuplicateLayer::Error)
    .output_compression(Compression::Gzip)
    .add_source(MvtSource::new("roads").with_layers(["roads"]))
    .add_source(
        MvtSource::new("buildings")
            .with_compression(Compression::Gzip)
            .with_layers(["building"]),
    )
    .build()?;

let response_body = composer.compose(&[roads, buildings_gzip])?;
assert_eq!(composer.output_compression().content_encoding(), Some("gzip"));
# let _ = response_body;
# Ok(())
# }
```

The source order is fixed by `add_source`. `compose(&inputs)` requires exactly that many inputs,
and `inputs[n]` always belongs to source `n`.

## Compression model

Input compression is source-fixed. Each `MvtSource` declares the format expected for that source's
request bytes; `compose` decodes every configured compressed input first, and the internal merge
only concatenates raw MVT bytes in source order. Raw inputs are borrowed during this preparation;
compressed inputs allocate decoded buffers.

The composer then returns the raw composite unchanged or applies its fixed output compression once
to the **complete** composite MVT. It never emits one compressed stream per source. Encoder settings
are not part of the public API: enabled codecs use their default parameters, and the output format
cannot be overridden per request.

For source metadata, `MvtSource::from_mvt` and `from_mvts` auto-detect only gzip and Zstandard
frames; other samples are treated as raw MVT. Brotli has no reliable signature and must always use
`from_mvt_with_compression` or `from_mvts_with_compression` with `Compression::Brotli`.

When returning compressed bytes through HTTP, set the response header from
`composer.output_compression().content_encoding()`:

| Output | `Content-Encoding` |
| --- | --- |
| `Compression::None` | omit the header |
| `Compression::Gzip` | `gzip` |
| `Compression::Zstd` | `zstd` |
| `Compression::Brotli` | `br` |

Gzip output is a single gzip member around the complete composite MVT. This crate creates bytes,
not HTTP responses, so the calling server owns headers and cache policy.

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `gzip` | yes | Enables gzip source decoding and complete-output encoding. |
| `zstd` | no | Enables Zstandard source decoding and complete-output encoding. |
| `brotli` | no | Enables explicitly configured Brotli source decoding and complete-output encoding. |

Selecting an unavailable codec is rejected while building the composer. `Compression::Other` is an
unsupported marker and is never a valid source or output configuration.

## Validation and errors

`MvtComposerBuilder::validate_duplicate_layers()` can be called independently and does not consume
or modify the builder. A repeated layer inside one source is always rejected. Between distinct
sources, `DuplicateLayer::Error` (the default) rejects the repeat and `DuplicateLayer::Allow`
accepts it. `build()` runs the same duplicate validation along with source ID, layer, and feature
checks.

Sample construction and source decoding return `SourceError`; configuration returns `BuildError`;
and request composition returns `ComposeError`. A decompression failure identifies the configured
source that failed. A compose failure never returns a partial composite.

## Concurrency and memory

After `build()`, `MvtComposer` is immutable and contains no mutex, cache, or request-level mutable
state. Share it with `Arc<MvtComposer>` across threads without library-managed locking. Each call
owns its decoded source buffers, one raw composite allocation, and, when compression is selected, a
separate final encoded buffer. Compression therefore briefly holds both raw and encoded composites;
this is an intentional first-release memory trade-off rather than streaming output.

## Example

Run the observable mixed-input example (enabled by the default `gzip` feature):

```text
cargo run --example mixed_sources
```

It prints `compression=gzip` and `layers=roads,pipeline,valve,building`, after reading the final
output back as MVT.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT)
at your option.
