# Composite MVT v1 Design

## 1. Purpose

`composite-mvt` is a publishable Rust library for combining Mapbox Vector Tile
(MVT) payloads from a fixed, validated set of sources. Static source metadata is
created once. Per-request tile bytes are supplied in the same fixed order.

The public `MvtComposer::compose` method accepts source payloads in their
configured compression formats, decompresses only where required, and returns
one uncompressed composite MVT. Its private raw composition step performs only
ordered byte concatenation.

The crate will be released under `MIT OR Apache-2.0` and prepared for publication
to crates.io.

## 2. Goals

Version 1 must:

- represent source IDs and layer names with strong newtypes;
- create sources explicitly or derive them from one or more sample tiles;
- use `fast-mvt 0.6.0` to validate sample MVTs and read layer names;
- support uncompressed, gzip, zstd, and Brotli source payloads;
- gate gzip, zstd, and Brotli decoders behind Cargo features;
- automatically detect uncompressed, gzip, and zstd sample tiles;
- accept Brotli samples through an explicit compression API;
- validate source IDs, layer names, duplicate layers, and decoder availability
  once when building a composer;
- preserve source order permanently;
- allow a single `compose` call for sources with different configured input
  compression formats;
- ensure the final concatenation step sees only uncompressed MVT bytes;
- allocate the final output once and copy each raw input exactly once;
- return immutable `bytes::Bytes`;
- allow lock-free sharing through `Arc<MvtComposer>`;
- include publish-quality documentation, examples, feature-matrix tests, and
  package validation.

## 3. Non-goals

Version 1 will not:

- merge features inside layers;
- rename duplicate layers;
- modify geometry, properties, or layer names;
- parse or encode MVT in the private raw composition step;
- make HTTP requests or responses;
- manage caches, retries, fallbacks, TileJSON, or configuration replacement;
- infer Brotli from a magic number, because the Brotli stream format has no
  reliable fixed signature;
- support `Compression::Other` inside a composer;
- return a compressed composite payload.

Applications that need an HTTP `Content-Encoding` must compress the final
composite payload as one stream. They must not concatenate independently
compressed gzip responses and serve the result as a single gzip-encoded HTTP
representation; browser network stacks do not reliably decode all gzip members.

## 4. Architecture

The design has four boundaries:

1. `MvtSource` stores the fixed ID, expected request compression, and layer set.
2. `MvtComposerBuilder` validates all static configuration once.
3. Public `MvtComposer::compose` checks input count and prepares each input by
   borrowing it when uncompressed or calling the source decoder when compressed.
4. Private `MvtComposer::compose_raw` concatenates prepared raw MVT bytes in
   source order.

```text
configured MvtSource list
        +
per-request input bytes
        |
        v
MvtComposer::compose
  - validate input count
  - None: borrow input
  - Gzip/Zstd/Brotli: decompress input
        |
        v
private compose_raw
  - checked total length
  - one output allocation
  - ordered byte copies
        |
        v
uncompressed composite MVT Bytes
```

`MvtComposer` is immutable after construction. It contains no mutex, read-write
lock, cache, or mutable request state.

## 5. Core types

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceId(Box<str>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayerName(Box<str>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Gzip,
    Zstd,
    Brotli,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicateLayer {
    Allow,
    Error,
}

#[derive(Debug, Clone)]
pub struct MvtSource {
    id: SourceId,
    compression: Compression,
    layers: Box<[LayerName]>,
}

pub struct MvtComposerBuilder {
    sources: Vec<MvtSource>,
    duplicate_layer: DuplicateLayer,
}

pub struct MvtComposer {
    sources: Box<[MvtSource]>,
}
```

`CompressionMode` is removed. Sources may have different input compression
formats because all compressed inputs are decoded before raw composition.

## 6. Public API

### 6.1 Source construction and inspection

Builder-style setters use `with_` names so getters can use the natural field
names.

```rust
impl MvtSource {
    pub fn new(id: impl Into<SourceId>) -> Self;

    pub fn with_compression(self, compression: Compression) -> Self;

    pub fn with_layers<I, S>(self, layers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<LayerName>;

    pub fn from_mvt(
        id: impl Into<SourceId>,
        bytes: &[u8],
    ) -> Result<Self, SourceError>;

    pub fn from_mvt_with_compression(
        id: impl Into<SourceId>,
        bytes: &[u8],
        compression: Compression,
    ) -> Result<Self, SourceError>;

    pub fn from_mvts<I, B>(
        id: impl Into<SourceId>,
        inputs: I,
    ) -> Result<Self, SourceError>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>;

    pub fn from_mvts_with_compression<I, B>(
        id: impl Into<SourceId>,
        inputs: I,
        compression: Compression,
    ) -> Result<Self, SourceError>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>;

    pub fn decompress<'a>(
        &self,
        input: &'a [u8],
    ) -> Result<std::borrow::Cow<'a, [u8]>, SourceError>;

    pub fn id(&self) -> &SourceId;
    pub fn compression(&self) -> Compression;
    pub fn layers(&self) -> &[LayerName];
}
```

`MvtSource::new` defaults to `Compression::None` and no layers. A source without
layers may exist temporarily but cannot be added to a successful composer.

`decompress` returns `Cow::Borrowed` for `Compression::None` and `Cow::Owned`
for enabled compressed formats. It returns an error for a disabled decoder,
invalid compressed input, or `Compression::Other`.

### 6.2 Automatic sample detection

`from_mvt` and `from_mvts` use this deterministic order:

1. Detect gzip by the RFC 1952 `1f 8b` signature.
2. Detect standard zstd frames by the RFC 8878 `28 b5 2f fd` signature and
   zstd skippable frames by their `50..5f 2a 4d 18` signature range.
3. Otherwise parse the bytes as an uncompressed MVT.

They do not guess Brotli. Brotli callers use the explicit compression variants.
The explicit `Compression::Other` value is rejected because the library has no
decoder for it.

All samples passed to automatic `from_mvts` must resolve to the same compression
format. Layer names are deduplicated while preserving first-observed order.

### 6.3 Composer construction

```rust
impl MvtComposer {
    pub fn builder() -> MvtComposerBuilder;
}

impl MvtComposerBuilder {
    pub fn duplicate_layer(self, behavior: DuplicateLayer) -> Self;
    pub fn add_source(self, source: MvtSource) -> Self;
    pub fn build(self) -> Result<MvtComposer, BuildError>;
}
```

`DuplicateLayer::Error` is the default. `build` validates:

1. at least one source exists;
2. source IDs are unique;
3. every source contains at least one layer;
4. layer names are non-empty;
5. layer names are unique within each source;
6. cross-source duplicate layers obey `DuplicateLayer`;
7. every configured compressed format has its Cargo decoder feature enabled;
8. no source uses `Compression::Other`.

Successful construction fixes the source order and produces an immutable
composer.

### 6.4 Composition

```rust
impl MvtComposer {
    pub fn sources(&self) -> &[MvtSource];

    pub fn compose<B>(&self, inputs: &[B]) -> Result<bytes::Bytes, ComposeError>
    where
        B: AsRef<[u8]>;

    fn compose_raw<B>(&self, raw_inputs: &[B]) -> Result<bytes::Bytes, ComposeError>
    where
        B: AsRef<[u8]>;
}
```

`compose` maps `inputs[n]` to `sources[n]`. It checks the count before doing any
work. Each source then prepares its corresponding input according to its fixed
compression metadata. If any decoder fails, composition stops without returning
a partial result.

`compose_raw` is private and receives only prepared, uncompressed MVT byte
slices. It uses checked length addition, one `BytesMut` allocation, ordered
`extend_from_slice` calls, and `freeze`.

The raw step deliberately does not validate protobuf or inspect layers. A
successfully decompressed but malformed request tile therefore remains the
caller's data-integrity responsibility.

## 7. Errors

All public errors derive `thiserror::Error` using `thiserror 2`. Decoder failures
retain an error source chain without exposing optional decoder crate types as
stable public API.

```rust
pub enum SourceError {
    EmptyBytes,
    CompressionFeatureDisabled { compression: Compression },
    UnsupportedCompression { compression: Compression },
    DecompressionFailed {
        compression: Compression,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    InvalidMvt,
    MissingLayerName,
    EmptyLayerName,
    NoLayers,
    InconsistentSampleCompression {
        expected: Compression,
        actual: Compression,
    },
}

pub enum BuildError {
    NoSources,
    DuplicateSourceId { id: SourceId },
    NoLayers { source_id: SourceId },
    EmptyLayerName { source_id: SourceId },
    DuplicateLayerName {
        layer: LayerName,
        first_source: SourceId,
        second_source: SourceId,
    },
    CompressionFeatureDisabled {
        source_id: SourceId,
        compression: Compression,
    },
    UnsupportedCompression {
        source_id: SourceId,
        compression: Compression,
    },
}

pub enum ComposeError {
    InputCountMismatch { expected: usize, actual: usize },
    SourceDecompression {
        source_id: SourceId,
        source: SourceError,
    },
    SizeOverflow,
}
```

Actual Rust definitions use explicit `#[error]` display messages and `#[source]`
attributes. Error messages include relevant source, layer, and compression
values.

## 8. Cargo features and dependencies

```toml
[dependencies]
bytes = "1"
thiserror = "2"
fast-mvt = { version = "0.6.0", default-features = false, features = ["reader"] }
flate2 = { version = "1.1.9", optional = true }
zstd = { version = "0.13.3", optional = true }
brotli = { version = "8.0.4", optional = true }

[features]
default = ["gzip"]
gzip = ["dep:flate2"]
zstd = ["dep:zstd"]
brotli = ["dep:brotli"]
```

These are the current stable compatible release lines selected for the initial
implementation. `Cargo.lock` records the exact resolved dependency graph, while
the published manifest keeps the normal semver requirements shown above.

The crate uses Rust 2024 Edition. Its declared MSRV is the highest MSRV required
by `fast-mvt 0.6.0` and the selected dependency set, but never lower than Rust
1.85. The resolved value is documented and tested before publication.

## 9. Modules

```text
src/
|- lib.rs
|- builder.rs
|- composer.rs
|- compression.rs
|- duplicate_layer.rs
|- error.rs
|- source.rs
`- source_reader/
   |- mod.rs
   |- mvt.rs
   |- gzip.rs
   |- zstd.rs
   `- brotli.rs
```

The source reader is used for sample inspection and for per-request source
decompression. The private raw composition implementation has no dependency on
`fast-mvt` or decoder modules.

## 10. Concurrency and configuration replacement

`MvtComposer` is composed only of immutable owned metadata and is `Send + Sync`
when its fields are. Applications share it as `Arc<MvtComposer>` without locks.
Each `compose` invocation owns its decompression buffers and final output.

Configuration replacement is an integration concern. An application may build
a new composer, validate it fully, and atomically replace an old `Arc` with
`arc-swap`. `arc-swap` is not a dependency of this crate.

## 11. Testing and acceptance

### 11.1 Source tests

- explicit uncompressed source construction;
- uncompressed, gzip, zstd, and Brotli sample parsing;
- gzip/zstd automatic detection;
- explicit Brotli parsing;
- empty and invalid bytes;
- no layers, missing names, empty names, and multiple layers;
- duplicate names in one sample;
- multi-sample union with stable first-observed order;
- inconsistent automatically detected sample compression;
- each disabled decoder feature;
- `Compression::None` returns borrowed bytes;
- compressed formats return owned decompressed bytes.

### 11.2 Builder tests

- no sources;
- duplicate source ID;
- source with no layers;
- empty layer name;
- duplicate layer within one source;
- duplicate layer across sources in `Allow` and `Error` modes;
- disabled compression feature;
- `Compression::Other`;
- successful construction and stable source order.

### 11.3 Composition tests

- one and multiple sources;
- too few and too many inputs;
- empty raw MVT bytes;
- source-order preservation;
- all-uncompressed inputs;
- mixed uncompressed, gzip, zstd, and Brotli inputs;
- source-specific decompression failure with source ID context;
- output independence and input immutability;
- size arithmetic is covered through a testable checked-length helper because
  allocating `usize::MAX` bytes is not a viable test;
- `Arc<MvtComposer>` shared by multiple threads with independent results.

### 11.4 End-to-end and Web compatibility tests

Tests build independent sample MVTs with the `fast-mvt` writer, encode them in
the configured source formats, call public `compose` once, and read the resulting
uncompressed composite with `fast-mvt`. All expected layers must be present in
the fixed order.

A Web compatibility test gzip-compresses the final composite as one gzip member,
decompresses it, and again verifies every layer. Documentation states that this
single final compression is the supported HTTP delivery pattern.

### 11.5 Verification matrix

Before release, the project must pass:

- `cargo fmt --check`;
- `cargo clippy --all-targets --all-features -- -D warnings`;
- tests with no default features;
- tests with default gzip;
- tests with each optional decoder independently;
- tests with all features;
- documentation tests;
- release build;
- `cargo package` or `cargo publish --dry-run` without publishing.

An example program serves as manual library QA: it creates sources with mixed
input compression, composes them, and proves through `fast-mvt` that every layer
is readable from the returned payload.

## 12. Publishability

The repository includes:

- `README.md` with quick start, feature table, compression semantics, concurrency
  example, and HTTP delivery warning;
- `LICENSE-MIT` and `LICENSE-APACHE`;
- complete crate metadata, categories, keywords, documentation URL, and MSRV;
- public rustdoc for all exported items and errors;
- a changelog starting at version `0.1.0`;
- no publication action as part of implementation unless separately requested.

Repository and homepage metadata are included only when a real public repository
URL is available; no placeholder URL is published.
