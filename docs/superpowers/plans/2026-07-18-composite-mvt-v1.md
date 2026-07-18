# Composite MVT v1 实施计划

> **供智能代理执行：** 必须使用 `superpowers:subagent-driven-development`（推荐）或 `superpowers:executing-plans`，按任务逐项实施。所有步骤使用复选框跟踪。

**目标：** 构建一个可发布到 crates.io 的 Rust 库，根据固定数据源配置解压请求输入、组合原始 MVT，并按 Composer 的固定输出格式返回未压缩、gzip、zstd 或 Brotli 结果。

**架构：** `MvtSource` 保存静态 ID、输入压缩格式和图层；`MvtComposerBuilder` 一次性校验全部配置；公开 `compose()` 先按 source 解压，私有 `compose_raw()` 只做一次分配和顺序复制，最后对完整结果执行可选的整体输出压缩。Composer 构建后不可变，可通过 `Arc` 无锁共享。

**技术栈：** Rust 2024 Edition、`fast-mvt 0.6.0`、`bytes 1`、`thiserror 2`、`flate2 1.1.9`、`zstd 0.13.3`、`brotli 8.0.4`。

## 全局约束

- 包名和 crate 名分别为 `composite-mvt` 与 `composite_mvt`，首版版本为 `0.1.0`。
- 许可证为 `MIT OR Apache-2.0`，仓库包含 `LICENSE-MIT` 和 `LICENSE-APACHE`。
- Rust Edition 为 2024；`fast-mvt 0.6.0` 的 MSRV 是 1.87，因此 `rust-version = "1.87"`。
- 默认 feature 只有 `gzip`；`zstd` 和 `brotli` 按需启用。
- 不增加设计规范之外的运行时依赖。
- `compose_raw()` 不允许调用 `fast-mvt`、解码器或编码器。
- `Compression::None` 输入使用借用，不产生中间复制。
- `Compression::Other` 可以描述独立 source，但不能进入成功构建的 Composer，也不能作为输出格式。
- 输出压缩使用依赖库默认参数；首版不公开压缩级别。
- 每个提交必须遵循仓库 Lore Commit Protocol。
- 规范来源：`docs/superpowers/specs/2026-07-18-composite-mvt-design.md`。

---

## 文件结构

```text
Cargo.toml                         crate 元数据、依赖和 feature
.gitignore                        忽略 target 与本地代码索引
LICENSE-MIT                       MIT 许可证
LICENSE-APACHE                    Apache-2.0 许可证
README.md                         用户文档与快速开始
CHANGELOG.md                      0.1.0 变更记录
src/lib.rs                        模块声明与稳定公开 re-export
src/compression.rs                压缩检测、输入解压、整体输出压缩
src/duplicate_layer.rs            DuplicateLayer 策略
src/error.rs                      thiserror 错误类型
src/source.rs                     SourceId、LayerName、MvtSource 与样本解析
src/builder.rs                    Builder 校验与 Composer 构造
src/composer.rs                   compose、compose_raw 与输出压缩流程
tests/common/mod.rs                生成 MVT 与压缩测试数据
tests/source_parsing.rs           样本解析和 feature 行为
tests/builder_validation.rs       Builder 与重复图层校验
tests/composition.rs              组合、输出格式、并发和端到端验证
examples/mixed_sources.rs         手工 QA 示例
```

`source_reader/` 不单独建目录：第一版将格式检测、三个短解码分支和输出编码统一放在 `compression.rs`，不在实施过程中改变这一文件边界。

---

### Task 1：初始化发布级 crate 与基础公开类型

**文件：**
- 创建：`Cargo.toml`
- 创建：`.gitignore`
- 创建：`LICENSE-MIT`
- 创建：`LICENSE-APACHE`
- 创建：`README.md`
- 创建：`src/lib.rs`
- 创建：`src/duplicate_layer.rs`
- 创建：`src/error.rs`
- 创建：`src/source.rs`
- 创建：`src/compression.rs`
- 创建：`src/builder.rs`
- 创建：`src/composer.rs`

**接口：**
- 产出：`SourceId`、`LayerName`、`Compression`、`DuplicateLayer`、`SourceError`、`BuildError`、`ComposeError`。
- 后续依赖：所有后续任务使用这些精确名称，不再定义同义类型。

- [ ] **步骤 1：创建 Cargo 清单和模块骨架**

`Cargo.toml` 使用以下内容：

```toml
[package]
name = "composite-mvt"
version = "0.1.0"
edition = "2024"
rust-version = "1.87"
description = "Compose fixed Mapbox Vector Tile sources with optional input and output compression"
license = "MIT OR Apache-2.0"
readme = "README.md"
keywords = ["mvt", "mapbox", "vector-tile", "tiles", "geospatial"]
categories = ["compression", "data-structures", "encoding"]

[dependencies]
bytes = "1"
thiserror = "2"
fast-mvt = { version = "0.6.0", default-features = false, features = ["reader"] }
flate2 = { version = "1.1.9", optional = true }
zstd = { version = "0.13.3", optional = true }
brotli = { version = "8.0.4", optional = true }

[dev-dependencies]
fast-mvt = { version = "0.6.0", default-features = false, features = ["reader", "writer"] }

[features]
default = ["gzip"]
gzip = ["dep:flate2"]
zstd = ["dep:zstd"]
brotli = ["dep:brotli"]
```

`.gitignore` 精确包含：

```gitignore
/target/
/.codegraph/
```

`src/lib.rs` 先声明全部模块，并只 re-export 已存在的类型：

```rust
mod builder;
mod composer;
mod compression;
mod duplicate_layer;
mod error;
mod source;

pub use duplicate_layer::DuplicateLayer;
pub use error::{BuildError, ComposeError, SourceError};
pub use source::{Compression, LayerName, MvtSource, SourceId};
```

其余四个模块先创建为空文件，确保模块路径固定。许可证文件使用 SPDX 对应的官方 MIT 与 Apache License 2.0 全文，不写自定义许可证摘要。

初始 `README.md` 精确包含：

```markdown
# composite-mvt

Rust library for composing fixed Mapbox Vector Tile sources.
```

- [ ] **步骤 2：编写基础类型失败测试**

在 `src/source.rs` 添加：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strong_names_convert_from_strings_and_display() {
        let source = SourceId::from("roads");
        let layer = LayerName::from(String::from("road_labels"));

        assert_eq!(source.as_ref(), "roads");
        assert_eq!(source.to_string(), "roads");
        assert_eq!(layer.as_ref(), "road_labels");
        assert_eq!(layer.to_string(), "road_labels");
    }

    #[test]
    fn compression_names_match_content_encodings() {
        assert_eq!(Compression::None.content_encoding(), None);
        assert_eq!(Compression::Gzip.content_encoding(), Some("gzip"));
        assert_eq!(Compression::Zstd.content_encoding(), Some("zstd"));
        assert_eq!(Compression::Brotli.content_encoding(), Some("br"));
        assert_eq!(Compression::Other.content_encoding(), None);
    }
}
```

- [ ] **步骤 3：运行测试并确认失败**

运行：`cargo test source::tests --no-default-features`

预期：编译失败，提示 `SourceId`、`LayerName`、`Compression` 尚未定义。

- [ ] **步骤 4：实现基础类型与重复策略**

在 `src/source.rs` 实现 `SourceId`、`LayerName` 和 `Compression`：

```rust
use std::fmt;

macro_rules! string_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(Box<str>);

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.into())
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value.into_boxed_str())
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

string_newtype!(SourceId);
string_newtype!(LayerName);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Gzip,
    Zstd,
    Brotli,
    Other,
}

impl Compression {
    #[must_use]
    pub const fn content_encoding(self) -> Option<&'static str> {
        match self {
            Self::None | Self::Other => None,
            Self::Gzip => Some("gzip"),
            Self::Zstd => Some("zstd"),
            Self::Brotli => Some("br"),
        }
    }
}

impl fmt::Display for Compression {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::None => "none",
            Self::Gzip => "gzip",
            Self::Zstd => "zstd",
            Self::Brotli => "brotli",
            Self::Other => "other",
        })
    }
}
```

在 `src/duplicate_layer.rs` 实现：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DuplicateLayer {
    Allow,
    #[default]
    Error,
}
```

- [ ] **步骤 5：定义 thiserror 错误外形**

在 `src/error.rs` 定义完整稳定外形；解码细节和转换在后续任务补充：

```rust
use std::error::Error;

use thiserror::Error;

use crate::{Compression, LayerName, SourceId};

pub(crate) type BoxError = Box<dyn Error + Send + Sync + 'static>;

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("MVT input is empty")]
    EmptyBytes,
    #[error("no MVT samples were supplied")]
    NoSamples,
    #[error("the {compression} Cargo feature is disabled")]
    CompressionFeatureDisabled { compression: Compression },
    #[error("unsupported compression format: {compression}")]
    UnsupportedCompression { compression: Compression },
    #[error("failed to decompress {compression} input")]
    DecompressionFailed {
        compression: Compression,
        #[source]
        source: BoxError,
    },
    #[error("input is not a valid MVT")]
    InvalidMvt,
    #[error("MVT layer name is missing")]
    MissingLayerName,
    #[error("MVT layer name is empty")]
    EmptyLayerName,
    #[error("MVT contains no layers")]
    NoLayers,
    #[error("sample compression mismatch: expected {expected}, got {actual}")]
    InconsistentSampleCompression {
        expected: Compression,
        actual: Compression,
    },
}

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("composer requires at least one source")]
    NoSources,
    #[error("duplicate source id: {id}")]
    DuplicateSourceId { id: SourceId },
    #[error("source `{source_id}` has no layers")]
    NoLayers { source_id: SourceId },
    #[error("source `{source_id}` contains an empty layer name")]
    EmptyLayerName { source_id: SourceId },
    #[error("layer `{layer}` is duplicated between `{first_source}` and `{second_source}`")]
    DuplicateLayerName {
        layer: LayerName,
        first_source: SourceId,
        second_source: SourceId,
    },
    #[error("source `{source_id}` requires disabled {compression} support")]
    CompressionFeatureDisabled {
        source_id: SourceId,
        compression: Compression,
    },
    #[error("source `{source_id}` uses unsupported compression {compression}")]
    UnsupportedCompression {
        source_id: SourceId,
        compression: Compression,
    },
    #[error("output requires disabled {compression} support")]
    OutputCompressionFeatureDisabled { compression: Compression },
    #[error("unsupported output compression: {compression}")]
    UnsupportedOutputCompression { compression: Compression },
}

#[derive(Debug, Error)]
pub enum ComposeError {
    #[error("input count mismatch: expected {expected}, got {actual}")]
    InputCountMismatch { expected: usize, actual: usize },
    #[error("failed to decompress source `{source_id}`")]
    SourceDecompression {
        source_id: SourceId,
        #[source]
        source: SourceError,
    },
    #[error("failed to compress composite output as {compression}")]
    OutputCompression {
        compression: Compression,
        #[source]
        source: BoxError,
    },
    #[error("composite MVT size overflow")]
    SizeOverflow,
}
```

- [ ] **步骤 6：运行基础测试和格式检查**

运行：`cargo test source::tests --no-default-features`

预期：2 个测试通过。

运行：`cargo fmt --check`

预期：退出码 0。

- [ ] **步骤 7：提交基础骨架**

```powershell
git add Cargo.toml .gitignore LICENSE-MIT LICENSE-APACHE README.md src
git commit -m "Create a stable public foundation for MVT composition" -m "Constraint: Rust 1.87 is required by fast-mvt 0.6.0" -m "Confidence: high" -m "Scope-risk: narrow" -m "Tested: cargo test source::tests --no-default-features; cargo fmt --check"
```

---

### Task 2：实现压缩检测、输入解压和整体输出编码

**文件：**
- 修改：`src/compression.rs`
- 修改：`src/source.rs`
- 测试：`src/compression.rs`

**接口：**
- 消费：`Compression`、`SourceError`、`BoxError`。
- 产出：`detect_compression(&[u8]) -> Compression`、`decompress(Compression, &[u8]) -> Result<Cow<[u8]>, SourceError>`、`compress(Compression, &[u8]) -> Result<Bytes, BoxError>`、`feature_enabled(Compression) -> bool`。

- [ ] **步骤 1：添加格式检测失败测试**

在 `src/compression.rs` 添加测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_known_frame_signatures() {
        assert_eq!(detect_compression(&[0x1f, 0x8b, 0x08]), Compression::Gzip);
        assert_eq!(
            detect_compression(&[0x28, 0xb5, 0x2f, 0xfd]),
            Compression::Zstd
        );
        assert_eq!(
            detect_compression(&[0x50, 0x2a, 0x4d, 0x18]),
            Compression::Zstd
        );
        assert_eq!(detect_compression(&[0x1a, 0x00]), Compression::None);
    }

    #[test]
    fn none_decompression_borrows_input() {
        let input = b"raw";
        let output = decompress(Compression::None, input).unwrap();
        assert!(matches!(output, Cow::Borrowed(_)));
        assert_eq!(output.as_ref(), input);
    }
}
```

- [ ] **步骤 2：确认测试失败**

运行：`cargo test compression::tests --no-default-features`

预期：编译失败，提示检测和解压函数不存在。

- [ ] **步骤 3：实现检测和 feature 查询**

在 `src/compression.rs` 添加：

```rust
use std::borrow::Cow;
use bytes::Bytes;

use crate::error::BoxError;
use crate::{Compression, SourceError};

pub(crate) fn detect_compression(input: &[u8]) -> Compression {
    let zstd_skippable = input.get(..4).is_some_and(|prefix| {
        (0x50..=0x5f).contains(&prefix[0])
            && prefix[1..] == [0x2a, 0x4d, 0x18]
    });
    if input.starts_with(&[0x1f, 0x8b]) {
        Compression::Gzip
    } else if input.starts_with(&[0x28, 0xb5, 0x2f, 0xfd])
        || zstd_skippable
    {
        Compression::Zstd
    } else {
        Compression::None
    }
}

pub(crate) const fn feature_enabled(compression: Compression) -> bool {
    match compression {
        Compression::None => true,
        Compression::Gzip => cfg!(feature = "gzip"),
        Compression::Zstd => cfg!(feature = "zstd"),
        Compression::Brotli => cfg!(feature = "brotli"),
        Compression::Other => false,
    }
}
```

- [ ] **步骤 4：实现 feature-gated 解压**

实现公共分派和每个条件编译分支：

```rust
pub(crate) fn decompress(
    compression: Compression,
    input: &[u8],
) -> Result<Cow<'_, [u8]>, SourceError> {
    match compression {
        Compression::None => Ok(Cow::Borrowed(input)),
        Compression::Gzip => decompress_gzip(input).map(Cow::Owned),
        Compression::Zstd => decompress_zstd(input).map(Cow::Owned),
        Compression::Brotli => decompress_brotli(input).map(Cow::Owned),
        Compression::Other => Err(SourceError::UnsupportedCompression { compression }),
    }
}

#[cfg(any(feature = "gzip", feature = "zstd", feature = "brotli"))]
fn decode_failure(compression: Compression, source: impl std::error::Error + Send + Sync + 'static) -> SourceError {
    SourceError::DecompressionFailed {
        compression,
        source: Box::new(source),
    }
}

#[cfg(feature = "gzip")]
fn decompress_gzip(input: &[u8]) -> Result<Vec<u8>, SourceError> {
    use std::io::Read as _;
    let mut output = Vec::new();
    flate2::read::GzDecoder::new(input)
        .read_to_end(&mut output)
        .map_err(|error| decode_failure(Compression::Gzip, error))?;
    Ok(output)
}

#[cfg(not(feature = "gzip"))]
fn decompress_gzip(_: &[u8]) -> Result<Vec<u8>, SourceError> {
    Err(SourceError::CompressionFeatureDisabled {
        compression: Compression::Gzip,
    })
}

#[cfg(feature = "zstd")]
fn decompress_zstd(input: &[u8]) -> Result<Vec<u8>, SourceError> {
    zstd::stream::decode_all(input)
        .map_err(|error| decode_failure(Compression::Zstd, error))
}

#[cfg(not(feature = "zstd"))]
fn decompress_zstd(_: &[u8]) -> Result<Vec<u8>, SourceError> {
    Err(SourceError::CompressionFeatureDisabled {
        compression: Compression::Zstd,
    })
}

#[cfg(feature = "brotli")]
fn decompress_brotli(input: &[u8]) -> Result<Vec<u8>, SourceError> {
    use std::io::Read as _;
    let mut output = Vec::new();
    brotli::Decompressor::new(input, 4096)
        .read_to_end(&mut output)
        .map_err(|error| decode_failure(Compression::Brotli, error))?;
    Ok(output)
}

#[cfg(not(feature = "brotli"))]
fn decompress_brotli(_: &[u8]) -> Result<Vec<u8>, SourceError> {
    Err(SourceError::CompressionFeatureDisabled {
        compression: Compression::Brotli,
    })
}
```

- [ ] **步骤 5：实现整体输出编码**

```rust
pub(crate) fn compress(compression: Compression, input: &[u8]) -> Result<Bytes, BoxError> {
    match compression {
        Compression::None => Ok(Bytes::copy_from_slice(input)),
        Compression::Gzip => compress_gzip(input).map(Bytes::from),
        Compression::Zstd => compress_zstd(input).map(Bytes::from),
        Compression::Brotli => compress_brotli(input).map(Bytes::from),
        Compression::Other => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "unsupported output compression",
        ))),
    }
}

#[cfg(feature = "gzip")]
fn compress_gzip(input: &[u8]) -> Result<Vec<u8>, BoxError> {
    use std::io::Write as _;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(input)?;
    Ok(encoder.finish()?)
}

#[cfg(not(feature = "gzip"))]
fn compress_gzip(_: &[u8]) -> Result<Vec<u8>, BoxError> {
    Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "gzip feature is disabled",
    )))
}

#[cfg(feature = "zstd")]
fn compress_zstd(input: &[u8]) -> Result<Vec<u8>, BoxError> {
    Ok(zstd::stream::encode_all(input, 0)?)
}

#[cfg(not(feature = "zstd"))]
fn compress_zstd(_: &[u8]) -> Result<Vec<u8>, BoxError> {
    Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "zstd feature is disabled",
    )))
}

#[cfg(feature = "brotli")]
fn compress_brotli(input: &[u8]) -> Result<Vec<u8>, BoxError> {
    let mut output = Vec::new();
    let params = brotli::enc::BrotliEncoderParams::default();
    brotli::BrotliCompress(&mut std::io::Cursor::new(input), &mut output, &params)?;
    Ok(output)
}

#[cfg(not(feature = "brotli"))]
fn compress_brotli(_: &[u8]) -> Result<Vec<u8>, BoxError> {
    Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "brotli feature is disabled",
    )))
}
```

实现时将 `Compression::None` 的输出复制优化留给 Composer：Composer 在 `None` 输出时直接返回 raw `Bytes`，不调用 `compress()`。

- [ ] **步骤 6：添加各 feature 往返测试**

在 `compression.rs` 的测试模块加入三个明确测试：

```rust
#[cfg(feature = "gzip")]
#[test]
fn gzip_round_trip() {
    let encoded = compress(Compression::Gzip, b"tile").unwrap();
    assert!(encoded.starts_with(&[0x1f, 0x8b]));
    assert_eq!(
        decompress(Compression::Gzip, &encoded).unwrap().as_ref(),
        b"tile"
    );
}

#[cfg(feature = "zstd")]
#[test]
fn zstd_round_trip() {
    let encoded = compress(Compression::Zstd, b"tile").unwrap();
    assert!(encoded.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]));
    assert_eq!(
        decompress(Compression::Zstd, &encoded).unwrap().as_ref(),
        b"tile"
    );
}

#[cfg(feature = "brotli")]
#[test]
fn brotli_round_trip() {
    let encoded = compress(Compression::Brotli, b"tile").unwrap();
    assert_eq!(
        decompress(Compression::Brotli, &encoded).unwrap().as_ref(),
        b"tile"
    );
}
```

- [ ] **步骤 7：运行 feature 矩阵并提交**

运行：

```powershell
cargo test compression::tests --no-default-features
cargo test compression::tests
cargo test compression::tests --no-default-features --features zstd
cargo test compression::tests --no-default-features --features brotli
cargo test compression::tests --all-features
```

预期：每条命令退出码为 0；对应 feature 的往返测试运行，未启用的测试不参与编译。

```powershell
git add src/compression.rs src/source.rs
git commit -m "Centralize compression boundaries around complete MVT payloads" -m "Constraint: Brotli has no reliable magic number and must be explicit on input" -m "Confidence: high" -m "Scope-risk: moderate" -m "Tested: compression feature matrix"
```

---

### Task 3：实现 MvtSource 显式构造、样本解析和多样本并集

**文件：**
- 修改：`src/source.rs`
- 测试：`tests/common/mod.rs`
- 测试：`tests/source_parsing.rs`

**接口：**
- 消费：任务 2 的 `detect_compression` 与 `decompress`。
- 产出：设计规范 6.1 中的全部 `MvtSource` 方法。

- [ ] **步骤 1：创建真实 MVT 测试辅助函数**

`tests/common/mod.rs`：

```rust
use fast_mvt::MvtTileBuilder;

pub fn tile_with_layers(names: &[&str]) -> Vec<u8> {
    let mut tile = MvtTileBuilder::new();
    for name in names {
        tile = tile.layer(*name).unwrap().end();
    }
    tile.encode()
}
```

- [ ] **步骤 2：编写 source 解析失败测试**

`tests/source_parsing.rs`：

```rust
mod common;

use common::tile_with_layers;
use composite_mvt::{Compression, MvtSource, SourceError};

#[test]
fn parses_uncompressed_layers_in_order() {
    let bytes = tile_with_layers(&["pipeline", "valve"]);
    let source = MvtSource::from_mvt("network", &bytes).unwrap();

    assert_eq!(source.id().as_ref(), "network");
    assert_eq!(source.compression(), Compression::None);
    assert_eq!(
        source.layers().iter().map(AsRef::as_ref).collect::<Vec<_>>(),
        ["pipeline", "valve"]
    );
}

#[test]
fn unions_multiple_samples_in_first_seen_order() {
    let first = tile_with_layers(&["pipeline", "valve"]);
    let second = tile_with_layers(&["pipeline", "station"]);
    let source = MvtSource::from_mvts("network", [&first, &second]).unwrap();

    assert_eq!(
        source.layers().iter().map(AsRef::as_ref).collect::<Vec<_>>(),
        ["pipeline", "valve", "station"]
    );
}

#[test]
fn rejects_empty_input_and_empty_sample_set() {
    assert!(matches!(
        MvtSource::from_mvt("empty", &[]),
        Err(SourceError::EmptyBytes)
    ));
    assert!(matches!(
        MvtSource::from_mvts::<Vec<&[u8]>, &[u8]>("empty", Vec::new()),
        Err(SourceError::NoSamples)
    ));
}

#[test]
fn rejects_invalid_mvt_and_inconsistent_sample_compression() {
    assert!(matches!(
        MvtSource::from_mvt("invalid", b"not-an-mvt"),
        Err(SourceError::InvalidMvt)
    ));

    #[cfg(feature = "gzip")]
    {
        let raw = tile_with_layers(&["roads"]);
        let encoded = common::gzip(&raw);
        assert!(matches!(
            MvtSource::from_mvts("roads", [&raw, &encoded]),
            Err(SourceError::InconsistentSampleCompression { .. })
        ));
    }
}
```

- [ ] **步骤 3：运行测试并确认失败**

运行：`cargo test --test source_parsing --no-default-features`

预期：编译失败，提示 `MvtSource` 方法不存在。

- [ ] **步骤 4：实现 MvtSource 与显式构造器**

在 `src/source.rs` 添加结构和 getter：

```rust
use std::borrow::Cow;
use std::collections::HashSet;

use fast_mvt::{MvtError, MvtReaderRef};

use crate::compression::{decompress, detect_compression};
use crate::SourceError;

#[derive(Debug, Clone)]
pub struct MvtSource {
    id: SourceId,
    compression: Compression,
    layers: Box<[LayerName]>,
}

impl MvtSource {
    pub fn new(id: impl Into<SourceId>) -> Self {
        Self {
            id: id.into(),
            compression: Compression::None,
            layers: Box::new([]),
        }
    }

    #[must_use]
    pub fn with_compression(mut self, compression: Compression) -> Self {
        self.compression = compression;
        self
    }

    pub fn with_layers<I, S>(mut self, layers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<LayerName>,
    {
        self.layers = layers.into_iter().map(Into::into).collect();
        self
    }

    pub fn id(&self) -> &SourceId { &self.id }
    pub fn compression(&self) -> Compression { self.compression }
    pub fn layers(&self) -> &[LayerName] { &self.layers }

    pub fn decompress<'a>(&self, input: &'a [u8]) -> Result<Cow<'a, [u8]>, SourceError> {
        decompress(self.compression, input)
    }
}
```

实际提交前将三个单行 getter 交给 `cargo fmt` 展开为项目格式，不改变签名。

- [ ] **步骤 5：实现单样本解析**

```rust
impl MvtSource {
    pub fn from_mvt(
        id: impl Into<SourceId>,
        bytes: &[u8],
    ) -> Result<Self, SourceError> {
        if bytes.is_empty() {
            return Err(SourceError::EmptyBytes);
        }
        let compression = detect_compression(bytes);
        Self::from_mvt_with_compression(id, bytes, compression)
    }

    pub fn from_mvt_with_compression(
        id: impl Into<SourceId>,
        bytes: &[u8],
        compression: Compression,
    ) -> Result<Self, SourceError> {
        if bytes.is_empty() {
            return Err(SourceError::EmptyBytes);
        }
        let raw = decompress(compression, bytes)?;
        let layers = read_layers(&raw)?;
        Ok(Self {
            id: id.into(),
            compression,
            layers,
        })
    }
}

fn read_layers(bytes: &[u8]) -> Result<Box<[LayerName]>, SourceError> {
    let reader = MvtReaderRef::new(bytes).map_err(|error| match error {
        MvtError::MissingLayerName => SourceError::MissingLayerName,
        _ => SourceError::InvalidMvt,
    })?;
    let layers: Box<[LayerName]> = reader.layers().map(|layer| layer.name().into()).collect();
    if layers.is_empty() {
        return Err(SourceError::NoLayers);
    }
    Ok(layers)
}
```

- [ ] **步骤 6：实现多样本解析和稳定并集**

```rust
impl MvtSource {
    pub fn from_mvts<I, B>(id: impl Into<SourceId>, inputs: I) -> Result<Self, SourceError>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        let inputs: Vec<B> = inputs.into_iter().collect();
        let first = inputs.first().ok_or(SourceError::NoSamples)?;
        let expected = detect_compression(first.as_ref());
        for input in &inputs {
            let actual = detect_compression(input.as_ref());
            if actual != expected {
                return Err(SourceError::InconsistentSampleCompression { expected, actual });
            }
        }
        Self::from_mvts_with_compression(id, inputs, expected)
    }

    pub fn from_mvts_with_compression<I, B>(
        id: impl Into<SourceId>,
        inputs: I,
        compression: Compression,
    ) -> Result<Self, SourceError>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        let mut found_sample = false;
        let mut seen = HashSet::new();
        let mut layers = Vec::new();
        for input in inputs {
            found_sample = true;
            if input.as_ref().is_empty() {
                return Err(SourceError::EmptyBytes);
            }
            let raw = decompress(compression, input.as_ref())?;
            for layer in read_layers(&raw)? {
                if seen.insert(layer.clone()) {
                    layers.push(layer);
                }
            }
        }
        if !found_sample {
            return Err(SourceError::NoSamples);
        }
        Ok(Self {
            id: id.into(),
            compression,
            layers: layers.into_boxed_slice(),
        })
    }
}
```

- [ ] **步骤 7：添加压缩样本测试并运行**

在 `tests/common/mod.rs` 添加 gzip helper：

```rust
#[cfg(feature = "gzip")]
pub fn gzip(input: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(input).unwrap();
    encoder.finish().unwrap()
}
```

在 `tests/source_parsing.rs` 添加：

```rust
#[cfg(feature = "gzip")]
#[test]
fn automatically_parses_gzip_sample() {
    let encoded = common::gzip(&tile_with_layers(&["roads"]));
    let source = MvtSource::from_mvt("roads", &encoded).unwrap();
    assert_eq!(source.compression(), Compression::Gzip);
    assert_eq!(source.layers()[0].as_ref(), "roads");
}

#[cfg(feature = "zstd")]
#[test]
fn automatically_parses_zstd_sample() {
    let raw = tile_with_layers(&["roads"]);
    let encoded = zstd::stream::encode_all(&raw[..], 0).unwrap();
    let source = MvtSource::from_mvt("roads", &encoded).unwrap();
    assert_eq!(source.compression(), Compression::Zstd);
    assert_eq!(source.layers()[0].as_ref(), "roads");
}

#[cfg(feature = "brotli")]
#[test]
fn explicitly_parses_brotli_sample() {
    use std::io::Cursor;
    let raw = tile_with_layers(&["roads"]);
    let mut encoded = Vec::new();
    brotli::BrotliCompress(
        &mut Cursor::new(raw),
        &mut encoded,
        &brotli::enc::BrotliEncoderParams::default(),
    )
    .unwrap();
    let source = MvtSource::from_mvt_with_compression(
        "roads",
        &encoded,
        Compression::Brotli,
    )
    .unwrap();
    assert_eq!(source.compression(), Compression::Brotli);
    assert_eq!(source.layers()[0].as_ref(), "roads");
}
```

运行：

```powershell
cargo test --test source_parsing --no-default-features
cargo test --test source_parsing --all-features
```

预期：全部测试通过。

- [ ] **步骤 8：提交 source 能力**

```powershell
git add src/source.rs tests/common/mod.rs tests/source_parsing.rs
git commit -m "Derive stable source metadata from representative MVT samples" -m "Constraint: Brotli samples require an explicit compression format" -m "Confidence: high" -m "Scope-risk: moderate" -m "Tested: source parsing with no features and all features"
```

---

### Task 4：实现 Builder、独立重复图层校验和不可变 Composer

**文件：**
- 修改：`src/lib.rs`
- 修改：`src/builder.rs`
- 修改：`src/composer.rs`
- 测试：`tests/builder_validation.rs`

**接口：**
- 消费：`MvtSource`、`DuplicateLayer`、`BuildError`、`feature_enabled`。
- 产出：`MvtComposer::builder()`、全部 Builder 链式方法、`validate_duplicate_layers()`、`build()`、Composer getter。

- [ ] **步骤 1：编写 Builder 失败测试**

`tests/builder_validation.rs`：

```rust
use composite_mvt::{BuildError, Compression, DuplicateLayer, MvtComposer, MvtSource};

fn source(id: &str, layers: &[&str]) -> MvtSource {
    MvtSource::new(id).with_layers(layers.iter().copied())
}

#[test]
fn rejects_no_sources_and_duplicate_ids() {
    assert!(matches!(MvtComposer::builder().build(), Err(BuildError::NoSources)));
    let result = MvtComposer::builder()
        .add_source(source("roads", &["roads"]))
        .add_source(source("roads", &["labels"]))
        .build();
    assert!(matches!(result, Err(BuildError::DuplicateSourceId { .. })));
}

#[test]
fn explicit_duplicate_validation_matches_build() {
    let builder = MvtComposer::builder()
        .duplicate_layer(DuplicateLayer::Error)
        .add_source(source("a", &["shared"]))
        .add_source(source("b", &["shared"]));
    assert!(matches!(
        builder.validate_duplicate_layers(),
        Err(BuildError::DuplicateLayerName { .. })
    ));
    assert!(matches!(builder.build(), Err(BuildError::DuplicateLayerName { .. })));
}

#[test]
fn allows_cross_source_duplicates_when_configured() {
    let composer = MvtComposer::builder()
        .duplicate_layer(DuplicateLayer::Allow)
        .add_source(source("a", &["shared"]))
        .add_source(source("b", &["shared"]))
        .build()
        .unwrap();
    assert_eq!(composer.sources().len(), 2);
    assert_eq!(composer.output_compression(), Compression::None);
}
```

- [ ] **步骤 2：运行测试并确认失败**

运行：`cargo test --test builder_validation --no-default-features`

预期：编译失败，Builder 和 Composer 方法尚未实现。

- [ ] **步骤 3：定义 Builder 和 Composer 状态**

先在 `src/lib.rs` 增加稳定 re-export：

```rust
pub use builder::MvtComposerBuilder;
pub use composer::MvtComposer;
```

`src/composer.rs`：

```rust
use crate::{Compression, MvtComposerBuilder, MvtSource};

pub struct MvtComposer {
    pub(crate) sources: Box<[MvtSource]>,
    pub(crate) output_compression: Compression,
}

impl MvtComposer {
    #[must_use]
    pub fn builder() -> MvtComposerBuilder {
        MvtComposerBuilder::default()
    }

    #[must_use]
    pub fn sources(&self) -> &[MvtSource] {
        &self.sources
    }

    #[must_use]
    pub fn output_compression(&self) -> Compression {
        self.output_compression
    }
}
```

`src/builder.rs`：

```rust
use std::collections::{HashMap, HashSet};

use crate::compression::feature_enabled;
use crate::{BuildError, Compression, DuplicateLayer, LayerName, MvtComposer, MvtSource, SourceId};

pub struct MvtComposerBuilder {
    sources: Vec<MvtSource>,
    duplicate_layer: DuplicateLayer,
    output_compression: Compression,
}

impl Default for MvtComposerBuilder {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            duplicate_layer: DuplicateLayer::Error,
            output_compression: Compression::None,
        }
    }
}

impl MvtComposerBuilder {
    #[must_use]
    pub fn duplicate_layer(mut self, behavior: DuplicateLayer) -> Self {
        self.duplicate_layer = behavior;
        self
    }

    #[must_use]
    pub fn output_compression(mut self, compression: Compression) -> Self {
        self.output_compression = compression;
        self
    }

    #[must_use]
    pub fn add_source(mut self, source: MvtSource) -> Self {
        self.sources.push(source);
        self
    }
}
```

- [ ] **步骤 4：实现独立重复图层校验**

```rust
impl MvtComposerBuilder {
    pub fn validate_duplicate_layers(&self) -> Result<(), BuildError> {
        let mut global: HashMap<&LayerName, &SourceId> = HashMap::new();
        for source in &self.sources {
            let mut local = HashSet::new();
            for layer in source.layers() {
                if !local.insert(layer) {
                    return Err(BuildError::DuplicateLayerName {
                        layer: layer.clone(),
                        first_source: source.id().clone(),
                        second_source: source.id().clone(),
                    });
                }
                if let Some(first_source) = global.get(layer) {
                    if self.duplicate_layer == DuplicateLayer::Error {
                        return Err(BuildError::DuplicateLayerName {
                            layer: layer.clone(),
                            first_source: (*first_source).clone(),
                            second_source: source.id().clone(),
                        });
                    }
                } else {
                    global.insert(layer, source.id());
                }
            }
        }
        Ok(())
    }
}
```

- [ ] **步骤 5：实现全部 Builder 校验并复用重复校验**

```rust
impl MvtComposerBuilder {
    pub fn build(self) -> Result<MvtComposer, BuildError> {
        self.validate()?;
        Ok(MvtComposer {
            sources: self.sources.into_boxed_slice(),
            output_compression: self.output_compression,
        })
    }

    fn validate(&self) -> Result<(), BuildError> {
        if self.sources.is_empty() {
            return Err(BuildError::NoSources);
        }
        let mut ids = HashSet::new();
        for source in &self.sources {
            if !ids.insert(source.id()) {
                return Err(BuildError::DuplicateSourceId {
                    id: source.id().clone(),
                });
            }
            if source.layers().is_empty() {
                return Err(BuildError::NoLayers {
                    source_id: source.id().clone(),
                });
            }
            if source.layers().iter().any(|layer| layer.as_ref().is_empty()) {
                return Err(BuildError::EmptyLayerName {
                    source_id: source.id().clone(),
                });
            }
            validate_source_compression(source)?;
        }
        self.validate_duplicate_layers()?;
        validate_output_compression(self.output_compression)
    }
}

fn validate_source_compression(source: &MvtSource) -> Result<(), BuildError> {
    let compression = source.compression();
    if compression == Compression::Other {
        return Err(BuildError::UnsupportedCompression {
            source_id: source.id().clone(),
            compression,
        });
    }
    if !feature_enabled(compression) {
        return Err(BuildError::CompressionFeatureDisabled {
            source_id: source.id().clone(),
            compression,
        });
    }
    Ok(())
}

fn validate_output_compression(compression: Compression) -> Result<(), BuildError> {
    if compression == Compression::Other {
        return Err(BuildError::UnsupportedOutputCompression { compression });
    }
    if !feature_enabled(compression) {
        return Err(BuildError::OutputCompressionFeatureDisabled { compression });
    }
    Ok(())
}
```

- [ ] **步骤 6：添加剩余 Builder 校验测试**

在 `tests/builder_validation.rs` 添加：

```rust
#[test]
fn rejects_missing_and_empty_layers() {
    assert!(matches!(
        MvtComposer::builder().add_source(MvtSource::new("empty")).build(),
        Err(BuildError::NoLayers { .. })
    ));
    assert!(matches!(
        MvtComposer::builder()
            .add_source(source("empty", &[""]))
            .build(),
        Err(BuildError::EmptyLayerName { .. })
    ));
}

#[test]
fn same_source_duplicates_are_always_invalid() {
    let builder = MvtComposer::builder()
        .duplicate_layer(DuplicateLayer::Allow)
        .add_source(source("roads", &["roads", "roads"]));
    assert!(matches!(
        builder.validate_duplicate_layers(),
        Err(BuildError::DuplicateLayerName { .. })
    ));
}

#[test]
fn rejects_other_for_input_and_output() {
    assert!(matches!(
        MvtComposer::builder()
            .add_source(source("roads", &["roads"]).with_compression(Compression::Other))
            .build(),
        Err(BuildError::UnsupportedCompression { .. })
    ));
    assert!(matches!(
        MvtComposer::builder()
            .output_compression(Compression::Other)
            .add_source(source("roads", &["roads"]))
            .build(),
        Err(BuildError::UnsupportedOutputCompression { .. })
    ));
}

#[cfg(not(feature = "zstd"))]
#[test]
fn rejects_disabled_input_and_output_features() {
    assert!(matches!(
        MvtComposer::builder()
            .add_source(source("roads", &["roads"]).with_compression(Compression::Zstd))
            .build(),
        Err(BuildError::CompressionFeatureDisabled { .. })
    ));
    assert!(matches!(
        MvtComposer::builder()
            .output_compression(Compression::Zstd)
            .add_source(source("roads", &["roads"]))
            .build(),
        Err(BuildError::OutputCompressionFeatureDisabled { .. })
    ));
}

#[test]
fn preserves_source_order() {
    let composer = MvtComposer::builder()
        .add_source(source("roads", &["roads"]))
        .add_source(source("buildings", &["building"]))
        .build()
        .unwrap();
    assert_eq!(
        composer
            .sources()
            .iter()
            .map(|source| source.id().as_ref())
            .collect::<Vec<_>>(),
        ["roads", "buildings"]
    );
}
```

运行：

```powershell
cargo test --test builder_validation --no-default-features
cargo test --test builder_validation --all-features
```

预期：全部通过。

- [ ] **步骤 7：提交 Builder**

```powershell
git add src/lib.rs src/builder.rs src/composer.rs tests/builder_validation.rs
git commit -m "Prevent invalid source and output configurations from becoming composers" -m "Constraint: build and validate_duplicate_layers share one duplicate policy implementation" -m "Confidence: high" -m "Scope-risk: moderate" -m "Tested: builder validation without features and with all features"
```

---

### Task 5：实现原始拼接、输入解压与固定输出压缩

**文件：**
- 修改：`src/composer.rs`
- 测试：`tests/composition.rs`

**接口：**
- 消费：`MvtSource::decompress()`、`compression::compress()`。
- 产出：`MvtComposer::compose<B>(&self, &[B]) -> Result<Bytes, ComposeError>` 和私有 `compose_raw()`。

- [ ] **步骤 1：编写原始顺序拼接失败测试**

`tests/composition.rs`：

```rust
use composite_mvt::{ComposeError, MvtComposer, MvtSource};

fn composer(ids: &[&str]) -> MvtComposer {
    ids.iter().fold(MvtComposer::builder(), |builder, id| {
        builder.add_source(MvtSource::new(*id).with_layers([*id]))
    }).build().unwrap()
}

#[test]
fn composes_raw_inputs_in_source_order() {
    let composer = composer(&["a", "b", "c"]);
    let output = composer
        .compose(&[&b"first"[..], &b"second"[..], &b"third"[..]])
        .unwrap();
    assert_eq!(output.as_ref(), b"firstsecondthird");
}

#[test]
fn rejects_wrong_input_count_before_composition() {
    let composer = composer(&["a", "b"]);
    assert!(matches!(
        composer.compose(&[b"only"]),
        Err(ComposeError::InputCountMismatch { expected: 2, actual: 1 })
    ));
    assert!(matches!(
        composer.compose(&[&b"one"[..], &b"two"[..], &b"three"[..]]),
        Err(ComposeError::InputCountMismatch { expected: 2, actual: 3 })
    ));
}

#[test]
fn empty_inputs_are_preserved_without_mutating_sources() {
    let composer = composer(&["a", "b"]);
    let first = Vec::<u8>::new();
    let second = b"second".to_vec();
    let output = composer.compose(&[&first, &second]).unwrap();
    assert_eq!(output.as_ref(), b"second");
    assert!(first.is_empty());
    assert_eq!(second, b"second");
}
```

- [ ] **步骤 2：确认测试失败**

运行：`cargo test --test composition --no-default-features`

预期：编译失败，`compose()` 尚未定义。

- [ ] **步骤 3：实现 checked-length helper 与 compose_raw**

在 `src/composer.rs` 添加：

```rust
use std::borrow::Cow;

use bytes::{Bytes, BytesMut};

use crate::compression::compress;
use crate::{ComposeError, Compression};

fn checked_total_len<'a>(inputs: impl IntoIterator<Item = &'a [u8]>) -> Result<usize, ComposeError> {
    checked_total_from_lengths(inputs.into_iter().map(|input| input.len()))
}

fn checked_total_from_lengths(
    lengths: impl IntoIterator<Item = usize>,
) -> Result<usize, ComposeError> {
    lengths.into_iter().try_fold(0usize, |total, length| {
        total.checked_add(length).ok_or(ComposeError::SizeOverflow)
    })
}

impl MvtComposer {
    fn compose_raw<B>(&self, raw_inputs: &[B]) -> Result<Bytes, ComposeError>
    where
        B: AsRef<[u8]>,
    {
        let total = checked_total_len(raw_inputs.iter().map(AsRef::as_ref))?;
        let mut output = BytesMut::with_capacity(total);
        for input in raw_inputs {
            output.extend_from_slice(input.as_ref());
        }
        Ok(output.freeze())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_total_rejects_overflow() {
        assert!(matches!(
            checked_total_from_lengths([usize::MAX, 1]),
            Err(ComposeError::SizeOverflow)
        ));
    }
}
```

- [ ] **步骤 4：实现公开 compose 数据流**

```rust
impl MvtComposer {
    pub fn compose<B>(&self, inputs: &[B]) -> Result<Bytes, ComposeError>
    where
        B: AsRef<[u8]>,
    {
        if inputs.len() != self.sources.len() {
            return Err(ComposeError::InputCountMismatch {
                expected: self.sources.len(),
                actual: inputs.len(),
            });
        }

        let raw_inputs: Vec<Cow<'_, [u8]>> = self
            .sources
            .iter()
            .zip(inputs)
            .map(|(source, input)| {
                source
                    .decompress(input.as_ref())
                    .map_err(|error| ComposeError::SourceDecompression {
                        source_id: source.id().clone(),
                        source: error,
                    })
            })
            .collect::<Result<_, _>>()?;

        let raw = self.compose_raw(&raw_inputs)?;
        if self.output_compression == Compression::None {
            return Ok(raw);
        }
        compress(self.output_compression, &raw).map_err(|source| {
            ComposeError::OutputCompression {
                compression: self.output_compression,
                source,
            }
        })
    }
}
```

导入 `Compression`，删除未使用的 `SourceId` 导入。保持 `compose_raw` 私有。

- [ ] **步骤 5：增加解压错误上下文测试**

在 gzip feature 下构建一个 gzip source，传入非法 gzip 字节，断言：

```rust
#[cfg(feature = "gzip")]
#[test]
fn reports_the_source_that_failed_decompression() {
    let composer = MvtComposer::builder()
        .add_source(
            MvtSource::new("roads")
                .with_compression(composite_mvt::Compression::Gzip)
                .with_layers(["roads"]),
        )
        .build()
        .unwrap();
    let error = composer.compose(&[b"not-gzip"]).unwrap_err();
    assert!(matches!(
        error,
        ComposeError::SourceDecompression { source_id, .. }
            if source_id.as_ref() == "roads"
    ));
}
```

- [ ] **步骤 6：运行核心组合测试并提交**

运行：

```powershell
cargo test --test composition --no-default-features
cargo test --test composition --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

预期：全部退出码为 0。

```powershell
git add src/composer.rs tests/composition.rs
git commit -m "Keep raw MVT concatenation deterministic behind one compose call" -m "Constraint: compressed inputs are prepared before compose_raw and output encoding happens after it" -m "Confidence: high" -m "Scope-risk: moderate" -m "Tested: composition tests and clippy with all features"
```

---

### Task 6：完成端到端格式、并发和 Web 交付验证

**文件：**
- 修改：`tests/common/mod.rs`
- 修改：`tests/composition.rs`
- 创建：`examples/mixed_sources.rs`

**接口：**
- 消费：全部公开 API。
- 产出：从真实 MVT 输入到可读取 Composite MVT 的证据，以及可执行手工 QA 示例。

- [ ] **步骤 1：增加读取图层和按格式编码辅助函数**

在 `tests/common/mod.rs` 增加 `layer_names(&[u8]) -> Vec<String>`，使用：

```rust
pub fn layer_names(bytes: &[u8]) -> Vec<String> {
    fast_mvt::MvtReaderRef::new(bytes)
        .unwrap()
        .layers()
        .map(|layer| layer.name().to_owned())
        .collect()
}
```

再添加以下 feature-gated helper：

```rust
#[cfg(feature = "gzip")]
pub fn gunzip(input: &[u8]) -> Vec<u8> {
    use std::io::Read;
    let mut output = Vec::new();
    flate2::read::GzDecoder::new(input)
        .read_to_end(&mut output)
        .unwrap();
    output
}

#[cfg(feature = "zstd")]
pub fn zstd_encode(input: &[u8]) -> Vec<u8> {
    zstd::stream::encode_all(input, 0).unwrap()
}

#[cfg(feature = "zstd")]
pub fn zstd_decode(input: &[u8]) -> Vec<u8> {
    zstd::stream::decode_all(input).unwrap()
}

#[cfg(feature = "brotli")]
pub fn brotli_encode(input: &[u8]) -> Vec<u8> {
    use std::io::Cursor;
    let mut output = Vec::new();
    brotli::BrotliCompress(
        &mut Cursor::new(input),
        &mut output,
        &brotli::enc::BrotliEncoderParams::default(),
    )
    .unwrap();
    output
}

#[cfg(feature = "brotli")]
pub fn brotli_decode(input: &[u8]) -> Vec<u8> {
    use std::io::Read;
    let mut output = Vec::new();
    brotli::Decompressor::new(input, 4096)
        .read_to_end(&mut output)
        .unwrap();
    output
}
```

- [ ] **步骤 2：添加混合输入、未压缩输出端到端测试**

在 `tests/composition.rs` 添加：

```rust
#[cfg(all(feature = "gzip", feature = "zstd"))]
#[test]
fn mixed_input_encodings_preserve_all_layers() {
    let roads = common::tile_with_layers(&["roads"]);
    let pipeline = common::gzip(&common::tile_with_layers(&["pipeline", "valve"]));
    let buildings = common::zstd_encode(&common::tile_with_layers(&["building"]));
    let composer = MvtComposer::builder()
        .add_source(MvtSource::new("roads").with_layers(["roads"]))
        .add_source(
            MvtSource::new("pipeline")
                .with_compression(Compression::Gzip)
                .with_layers(["pipeline", "valve"]),
        )
        .add_source(
            MvtSource::new("buildings")
                .with_compression(Compression::Zstd)
                .with_layers(["building"]),
        )
        .build()
        .unwrap();
    let output = composer.compose(&[&roads, &pipeline, &buildings]).unwrap();
    assert_eq!(
        common::layer_names(&output),
        ["roads", "pipeline", "valve", "building"]
    );
}
```

- [ ] **步骤 3：添加三种整体输出压缩测试**

在 `tests/composition.rs` 添加三个测试：

```rust
fn raw_two_layer_composer(output: Compression) -> MvtComposer {
    MvtComposer::builder()
        .output_compression(output)
        .add_source(MvtSource::new("roads").with_layers(["roads"]))
        .add_source(MvtSource::new("buildings").with_layers(["building"]))
        .build()
        .unwrap()
}

#[cfg(feature = "gzip")]
#[test]
fn emits_one_complete_gzip_output() {
    let roads = common::tile_with_layers(&["roads"]);
    let buildings = common::tile_with_layers(&["building"]);
    let output = raw_two_layer_composer(Compression::Gzip)
        .compose(&[&roads, &buildings])
        .unwrap();
    assert!(output.starts_with(&[0x1f, 0x8b]));
    assert_eq!(common::layer_names(&common::gunzip(&output)), ["roads", "building"]);
}

#[cfg(feature = "zstd")]
#[test]
fn emits_one_complete_zstd_output() {
    let roads = common::tile_with_layers(&["roads"]);
    let buildings = common::tile_with_layers(&["building"]);
    let output = raw_two_layer_composer(Compression::Zstd)
        .compose(&[&roads, &buildings])
        .unwrap();
    assert!(output.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]));
    assert_eq!(common::layer_names(&common::zstd_decode(&output)), ["roads", "building"]);
}

#[cfg(feature = "brotli")]
#[test]
fn emits_one_complete_brotli_output() {
    let roads = common::tile_with_layers(&["roads"]);
    let buildings = common::tile_with_layers(&["building"]);
    let output = raw_two_layer_composer(Compression::Brotli)
        .compose(&[&roads, &buildings])
        .unwrap();
    assert_eq!(common::layer_names(&common::brotli_decode(&output)), ["roads", "building"]);
}
```

- [ ] **步骤 4：添加并发测试**

```rust
#[test]
fn arc_composer_is_safe_for_concurrent_requests() {
    use std::sync::Arc;
    use std::thread;

    let composer = Arc::new(
        MvtComposer::builder()
            .add_source(MvtSource::new("roads").with_layers(["roads"]))
            .build()
            .unwrap(),
    );
    let handles: Vec<_> = (0_u8..16)
        .map(|value| {
            let composer = Arc::clone(&composer);
            thread::spawn(move || composer.compose(&[vec![value]]).unwrap())
        })
        .collect();
    for (value, handle) in (0_u8..16).zip(handles) {
        assert_eq!(handle.join().unwrap().as_ref(), &[value]);
    }
}
```

另加编译期 helper：

```rust
fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn composer_is_send_and_sync() {
    assert_send_sync::<MvtComposer>();
}

#[cfg(feature = "gzip")]
#[test]
fn compressed_output_is_independent_across_threads() {
    use std::sync::Arc;
    use std::thread;

    let composer = Arc::new(
        MvtComposer::builder()
            .output_compression(Compression::Gzip)
            .add_source(MvtSource::new("roads").with_layers(["roads"]))
            .build()
            .unwrap(),
    );
    let handles: Vec<_> = (0_u8..8)
        .map(|value| {
            let composer = Arc::clone(&composer);
            thread::spawn(move || (value, composer.compose(&[vec![value]]).unwrap()))
        })
        .collect();
    for handle in handles {
        let (value, output) = handle.join().unwrap();
        assert_eq!(common::gunzip(&output), [value]);
    }
}
```

- [ ] **步骤 5：创建手工 QA 示例**

`examples/mixed_sources.rs` 使用以下完整入口和私有 helper：

```rust
#[cfg(feature = "gzip")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use composite_mvt::{Compression, MvtComposer, MvtSource};
    use fast_mvt::MvtReaderRef;

    let roads = tile_with_layers(&["roads"]);
    let pipeline = gzip(&tile_with_layers(&["pipeline", "valve"]));
    let building = tile_with_layers(&["building"]);
    let composer = MvtComposer::builder()
        .output_compression(Compression::Gzip)
        .add_source(MvtSource::new("roads").with_layers(["roads"]))
        .add_source(
            MvtSource::new("pipeline")
                .with_compression(Compression::Gzip)
                .with_layers(["pipeline", "valve"]),
        )
        .add_source(MvtSource::new("building").with_layers(["building"]))
        .build()?;
    let output = composer.compose(&[&roads, &pipeline, &building])?;
    let raw = gunzip(&output);
    let layers = MvtReaderRef::new(&raw)?
        .layers()
        .map(|layer| layer.name())
        .collect::<Vec<_>>()
        .join(",");
    println!("compression=gzip");
    println!("layers={layers}");
    Ok(())
}

#[cfg(not(feature = "gzip"))]
fn main() {
    println!("enable the gzip feature to run this example");
}

#[cfg(feature = "gzip")]
fn tile_with_layers(names: &[&str]) -> Vec<u8> {
    let mut tile = fast_mvt::MvtTileBuilder::new();
    for name in names {
        tile = tile.layer(*name).unwrap().end();
    }
    tile.encode()
}

#[cfg(feature = "gzip")]
fn gzip(input: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(input).unwrap();
    encoder.finish().unwrap()
}

#[cfg(feature = "gzip")]
fn gunzip(input: &[u8]) -> Vec<u8> {
    use std::io::Read;
    let mut output = Vec::new();
    flate2::read::GzDecoder::new(input)
        .read_to_end(&mut output)
        .unwrap();
    output
}
```

运行结果必须是：

```text
compression=gzip
layers=roads,pipeline,valve,building
```

- [ ] **步骤 6：运行端到端和手工 QA**

运行：

```powershell
cargo test --test composition --all-features
cargo run --example mixed_sources --all-features
```

预期：测试全部通过；示例输出精确包含 `compression=gzip` 和四个固定顺序图层。

- [ ] **步骤 7：提交端到端证据**

```powershell
git add tests/common/mod.rs tests/composition.rs examples/mixed_sources.rs
git commit -m "Prove mixed source and output encodings preserve every MVT layer" -m "Constraint: Web gzip output is one member around the complete composite" -m "Confidence: high" -m "Scope-risk: moderate" -m "Tested: all-feature integration tests and mixed_sources example"
```

---

### Task 7：完成 rustdoc、README、变更记录和发布验证

**文件：**
- 修改：`src/lib.rs`
- 修改：全部 `src/*.rs` 公开项 rustdoc
- 创建：`README.md`
- 创建：`CHANGELOG.md`
- 修改：`Cargo.toml`（仅在真实 repository URL 已知时添加 repository；否则不添加）

**接口：**
- 消费：全部稳定公开 API。
- 产出：可由新用户直接使用、可通过 `cargo package` 的 0.1.0 crate。

- [ ] **步骤 1：为公开 API 添加 rustdoc 与 crate 示例**

`src/lib.rs` 顶部增加 `#![doc = include_str!("../README.md")]` 和 `#![forbid(unsafe_code)]`。每个公开类型、variant 和方法说明：

- 静态配置与请求字节的边界；
- 自动检测不包含 Brotli；
- `validate_duplicate_layers()` 的 Allow/Error 语义；
- `compose()` 的固定输入顺序；
- 输出格式与 `Content-Encoding` 对应关系；
- gzip HTTP 输出必须是完整 Composite MVT 的单一 member。

- [ ] **步骤 2：编写 README 快速开始**

README 必须包含可编译示例：

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

README feature 表精确列出 `gzip` 默认启用、`zstd` 和 `brotli` 可选；说明 Brotli 输入必须显式声明；说明输出压缩采用默认参数且固定在 Composer 上。

- [ ] **步骤 3：编写 CHANGELOG**

`CHANGELOG.md` 使用 Keep a Changelog 风格，包含 `0.1.0 - 2026-07-18`，列出 source 解析、三种可选压缩、Builder 校验、重复图层方法、无锁 Composer、整体输出压缩与发布验证。

- [ ] **步骤 4：运行完整验证矩阵**

按顺序运行，任一失败立即修复后从受影响命令继续：

```powershell
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --no-default-features
cargo test
cargo test --no-default-features --features zstd
cargo test --no-default-features --features brotli
cargo test --all-features
cargo test --doc --all-features
cargo build --release --all-features
cargo package
```

预期：全部退出码为 0；`cargo package` 列出的内容包含许可证、README、CHANGELOG、源码、测试与示例，不包含 `.codegraph` 或 `target`。

- [ ] **步骤 5：安装并检查 MSRV**

运行：

```powershell
rustup toolchain install 1.87.0 --profile minimal
rustup run 1.87.0 cargo check --all-features
```

预期：两条命令退出码均为 0，且不修改 `rust-version = "1.87"`。

- [ ] **步骤 6：提交发布质量文档**

```powershell
git add Cargo.toml README.md CHANGELOG.md src
git commit -m "Make the first composite-mvt release understandable and packageable" -m "Constraint: No repository URL is published until a real public remote exists" -m "Confidence: high" -m "Scope-risk: narrow" -m "Tested: Rust 1.87 check, fmt, clippy, feature matrix, doctests, release build, cargo package"
```

---

## 最终完成条件

- [ ] 设计规范中的全部公开 API 存在且 rustdoc 完整。
- [ ] `validate_duplicate_layers()` 可独立调用，`build()` 复用同一逻辑。
- [ ] 输入 None/gzip/zstd/Brotli 与输出 None/gzip/zstd/Brotli 的支持符合 feature 配置。
- [ ] `compose_raw()` 只执行长度检查、一次分配和顺序复制。
- [ ] gzip 输出是包裹完整 Composite MVT 的单一 member。
- [ ] `fast-mvt` 能读取所有端到端结果中的固定顺序图层。
- [ ] `Arc<MvtComposer>` 并发测试通过，无锁且结果独立。
- [ ] 完整验证矩阵通过，或唯一未执行项是明确记录的 Rust 1.87 工具链缺失。
- [ ] `cargo package` 成功，未实际执行 `cargo publish`。
- [ ] 工作树只保留用户原有的无关变更，不包含构建产物。
