# Composite MVT v1 设计说明书

## 1. 文档目的

`composite-mvt` 是一个准备发布到 crates.io 的 Rust 库，用于将一组已经固定并完成校验的数据源所返回的 Mapbox Vector Tile（MVT）组合成一个新的 MVT。数据源的静态元数据只在初始化阶段创建一次；每次请求只需要按照固定顺序提供当前瓦片的字节数据。

公开方法 `MvtComposer::compose()` 接收各数据源原始或压缩后的输入，根据数据源配置执行必要的解压，在内部生成原始 Composite MVT，再按照 Composer 固定的输出压缩配置返回未压缩、gzip、zstd 或 Brotli 格式的结果。其内部的原始组合步骤只执行按顺序的字节拼接。

本项目使用 `MIT OR Apache-2.0` 双许可证，并按可发布到 crates.io 的通用开源库标准交付。

## 2. 第一版目标

第一版必须实现：

- 使用强类型 newtype 表示数据源 ID 和图层名称；
- 支持显式创建 `MvtSource`；
- 支持从单个或多个 MVT 样本中解析并创建 `MvtSource`；
- 使用 `fast-mvt 0.6.0` 校验样本 MVT 并读取图层名称；
- 支持未压缩、gzip、zstd 和 Brotli 输入；
- 通过 Cargo feature 分别控制 gzip、zstd 和 Brotli 解码器；
- 自动识别未压缩、gzip 和 zstd 样本；
- 通过显式压缩格式接口读取 Brotli 样本；
- 在构建 Composer 时一次性校验数据源 ID、图层名称、重名图层、解码器 feature 和压缩格式；
- 永久固定数据源顺序；
- 允许一次 `compose()` 调用处理采用不同输入压缩格式的数据源；
- 保证内部最终拼接步骤只接收已解压的原始 MVT；
- 原始拼接阶段只分配一次内存，并将每个原始输入复制一次；
- 允许在 Composer 上固定配置输出压缩格式；
- 对完整的原始 Composite MVT 进行一次整体输出压缩；
- 返回不可变的 `bytes::Bytes`；
- 支持通过 `Arc<MvtComposer>` 进行无锁并发共享；
- 提供达到发布质量的文档、示例、feature 组合测试和打包验证。

## 3. 第一版不支持的内容

第一版不支持：

- 合并同名图层中的 feature；
- 自动重命名图层；
- 修改 geometry、属性或图层名称；
- 在内部原始拼接阶段解析或编码 MVT；
- HTTP 请求或响应；
- 缓存、重试、降级、TileJSON 或配置替换；
- 通过 magic number 猜测 Brotli，因为 Brotli 流没有可靠的固定文件签名；
- 在 Composer 中使用 `Compression::Other`；
- 为输出压缩暴露压缩级别、Brotli quality/window 等调优参数。

如果应用需要设置 HTTP `Content-Encoding`，应选择 Composer 对完整 Composite MVT 进行一次整体输出压缩，再根据 `output_compression()` 设置对应响应头。不得将多个独立 gzip 数据直接拼接后作为一个 gzip HTTP 响应返回，因为浏览器网络栈不能可靠地解码其中的全部 gzip member。

## 4. 总体架构

系统划分为五个明确边界：

1. `MvtSource` 保存固定的数据源 ID、请求输入所采用的压缩格式和图层集合。
2. `MvtComposerBuilder` 在初始化阶段一次性校验全部静态配置。
3. 公开的 `MvtComposer::compose()` 校验输入数量；未压缩输入直接借用，压缩输入通过对应 `MvtSource` 解压。
4. 私有的 `MvtComposer::compose_raw()` 只将准备好的原始 MVT 按数据源顺序拼接。
5. 公开 `compose()` 根据 Composer 的固定输出配置，将完整原始结果直接返回或整体压缩一次。

```text
固定的 MvtSource 列表
        +
每次请求的输入 bytes
        │
        ▼
MvtComposer::compose()
  - 校验输入数量
  - None：直接借用输入
  - Gzip/Zstd/Brotli：解压输入
        │
        ▼
私有 compose_raw()
  - 检查并计算总长度
  - 一次性申请输出空间
  - 按顺序复制字节
        │
        ▼
未压缩的 Composite MVT
        │
        ▼
output_compression
  - None：直接返回原始 Bytes
  - Gzip/Zstd/Brotli：整体压缩一次
        │
        ▼
最终 Composite MVT Bytes
```

`MvtComposer` 构建完成后不可修改。它不包含 `Mutex`、`RwLock`、缓存或任何请求级可变状态。

## 5. 核心类型

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
    output_compression: Compression,
}

pub struct MvtComposer {
    sources: Box<[MvtSource]>,
    output_compression: Compression,
}
```

删除原需求中的 `CompressionMode`。不同数据源可以采用不同的输入压缩格式，因为全部压缩输入都会在原始拼接之前完成解压。输出压缩格式独立于各 source 的输入压缩格式，并在 Composer 构建完成后保持不变。

## 6. 公开 API

### 6.1 创建和读取 MvtSource

链式设置方法使用 `with_` 前缀，使 getter 可以直接使用自然的字段名称。

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

`MvtSource::new()` 默认使用 `Compression::None`，图层列表默认为空。没有图层的 source 可以作为尚未配置完成的临时对象存在，但不能构建出合法的 Composer。

`decompress()` 在 `Compression::None` 时返回 `Cow::Borrowed`，不执行复制；在启用相应 feature 的压缩格式下返回 `Cow::Owned`。如果解码器 feature 未启用、压缩数据无效或格式为 `Compression::Other`，则返回错误。

### 6.2 自动识别样本压缩格式

`from_mvt()` 和 `from_mvts()` 按以下固定顺序识别：

1. 使用 RFC 1952 的 `1f 8b` 签名识别 gzip；
2. 使用 RFC 8878 的 `28 b5 2f fd` 签名识别标准 zstd frame；
3. 使用 `50..5f 2a 4d 18` 签名范围识别 zstd skippable frame；
4. 未匹配已知签名时，按未压缩 MVT 进行解析。

自动接口不猜测 Brotli。Brotli 调用方必须使用显式压缩格式方法。`Compression::Other` 也不能用于解析，因为本库不知道对应的解码器。

自动版 `from_mvts()` 要求所有样本最终识别为相同压缩格式。多个样本的图层名称取并集，并保持第一次观察到该图层时的顺序。

### 6.3 构建 MvtComposer

```rust
impl MvtComposer {
    pub fn builder() -> MvtComposerBuilder;
}

impl MvtComposerBuilder {
    pub fn duplicate_layer(self, behavior: DuplicateLayer) -> Self;
    pub fn output_compression(self, compression: Compression) -> Self;
    pub fn add_source(self, source: MvtSource) -> Self;
    pub fn validate_duplicate_layers(&self) -> Result<(), BuildError>;
    pub fn build(self) -> Result<MvtComposer, BuildError>;
}
```

`validate_duplicate_layers()` 是可独立调用的公开校验方法：

- 同一个 source 内部出现重复图层时始终返回 `BuildError::DuplicateLayerName`；
- 不同 source 之间出现同名图层时，根据 `DuplicateLayer` 返回错误或允许通过；
- 方法只读取 Builder 当前状态，不消费或修改 Builder；
- `build()` 必须复用同一套内部实现，不得复制或绕过该校验逻辑。

`MvtComposer` 不提供重复图层校验方法，因为成功构建的 Composer 已经满足 Builder 建立的不变量。

`DuplicateLayer::Error` 是默认重名策略，`Compression::None` 是默认输出格式。`build()` 必须按以下全局遍次依次校验；前一遍必须扫描完全部 source 后，才能开始后一遍：

1. 至少存在一个 source；
2. 扫描全部 source，确认 source ID 不重复；
3. 扫描全部 source，确认每个 source 至少包含一个图层；
4. 扫描全部 source，确认图层名称非空；
5. 调用公开的 `validate_duplicate_layers()`，同时校验同 source 和跨 source 重名；
6. 扫描全部 source，先拒绝 `Compression::Other`，再确认所需 Cargo 解码器 feature 已启用；
7. 最后校验输出压缩，先拒绝 `Compression::Other`，再确认所需 Cargo feature 已启用。

构建成功后，source 顺序永久固定，并生成不可变的 `MvtComposer`。

### 6.4 组合 MVT

```rust
impl MvtComposer {
    pub fn sources(&self) -> &[MvtSource];
    pub fn output_compression(&self) -> Compression;

    pub fn compose<B>(&self, inputs: &[B]) -> Result<bytes::Bytes, ComposeError>
    where
        B: AsRef<[u8]>;

    fn compose_raw<B>(&self, raw_inputs: &[B]) -> Result<bytes::Bytes, ComposeError>
    where
        B: AsRef<[u8]>;
}
```

`compose()` 按索引建立固定对应关系：`inputs[n]` 对应 `sources[n]`。它首先校验输入数量，然后根据每个 source 的固定压缩元数据准备对应输入。任意解码失败都会立即终止组合，不返回部分结果。

`compose_raw()` 是私有方法，只接收已经准备好的未压缩 MVT。它使用 checked addition 计算总长度，通过 `BytesMut` 一次性申请输出空间，按顺序执行 `extend_from_slice()`，最后调用 `freeze()` 返回不可变 `Bytes`。

`compose_raw()` 返回完整的未压缩 Composite MVT 后，`compose()` 再根据 `output_compression` 处理输出：

- `Compression::None`：直接返回原始 `Bytes`，不复制；
- `Compression::Gzip`：使用 `flate2` 默认压缩级别，对完整结果生成一个 gzip member；
- `Compression::Zstd`：使用 `zstd` 默认压缩级别，对完整结果生成一个 zstd frame；
- `Compression::Brotli`：使用 `brotli` 默认参数，对完整结果生成一个 Brotli stream。

首版不允许请求级覆盖输出压缩格式，也不暴露压缩级别。调用方通过 `output_compression()` 查询结果编码，并可据此设置 HTTP `Content-Encoding`：gzip 对应 `gzip`，zstd 对应 `zstd`，Brotli 对应 `br`，`None` 不设置该响应头。本库只返回字节，不创建 HTTP 响应。

压缩输出会短暂同时持有未压缩 Composite MVT 和最终压缩结果。这是首版为保持实现边界清晰而接受的内存开销；流式写入输出压缩器属于经过性能测量后再考虑的优化。

原始拼接步骤有意不校验 protobuf，也不读取图层。已经成功解压、但请求数据本身不是合法 MVT 的情况属于调用方的数据完整性责任。

## 7. 错误设计

全部公开错误均使用 `thiserror 2` 派生 `thiserror::Error`。解码失败保留底层错误链，但不会将可选压缩依赖的具体错误类型固化为稳定公开 API。

```rust
pub enum SourceError {
    EmptyBytes,
    NoSamples,
    CompressionFeatureDisabled {
        compression: Compression,
    },
    UnsupportedCompression {
        compression: Compression,
    },
    DecompressionFailed {
        compression: Compression,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    InvalidMvt,
    MissingLayerName,
    NoLayers,
    InconsistentSampleCompression {
        expected: Compression,
        actual: Compression,
    },
}

pub enum BuildError {
    NoSources,
    DuplicateSourceId {
        id: SourceId,
    },
    NoLayers {
        source_id: SourceId,
    },
    EmptyLayerName {
        source_id: SourceId,
    },
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
    OutputCompressionFeatureDisabled {
        compression: Compression,
    },
    UnsupportedOutputCompression {
        compression: Compression,
    },
}

pub enum ComposeError {
    InputCountMismatch {
        expected: usize,
        actual: usize,
    },
    SourceDecompression {
        source_id: SourceId,
        source: SourceError,
    },
    OutputCompression {
        compression: Compression,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    SizeOverflow,
}
```

`fast-mvt 0.6.0` 对缺失 name 字段和显式编码为空字符串的 name 字段都返回 `MvtError::MissingLayerName`，因此 `SourceError::MissingLayerName` 同时覆盖这两种输入。显式创建的 `MvtSource` 仍由 Builder 使用 `BuildError::EmptyLayerName` 拒绝空图层名。

实际 Rust 定义为每个 variant 提供明确的 `#[error]` 展示消息，并通过 `#[source]` 连接底层错误。错误文本包含相关 source、layer 和 compression 上下文。

## 8. Cargo features 与依赖

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

以上版本是首版实现所选用的当前稳定兼容版本线。`Cargo.lock` 记录实际解析出的完整依赖图，发布清单则保留上述常规 semver 约束。

项目使用 Rust 2024 Edition。声明的最低 Rust 版本为 `fast-mvt 0.6.0` 与全部所选依赖实际要求中的最高值，但不会低于 Rust 1.85。实现阶段必须验证并记录最终 MSRV。

## 9. 模块结构

```text
src/
├── lib.rs
├── builder.rs
├── composer.rs
├── compression.rs
├── duplicate_layer.rs
├── error.rs
├── source.rs
└── source_reader/
    ├── mod.rs
    ├── mvt.rs
    ├── gzip.rs
    ├── zstd.rs
    └── brotli.rs
```

`source_reader` 同时服务于初始化阶段的样本读取和请求阶段的 source 解压。`compression.rs` 还负责对完整原始 Composite MVT 执行输出压缩。私有的原始组合实现不依赖 `fast-mvt` 或任何压缩编解码模块。

## 10. 并发与配置替换

`MvtComposer` 只包含不可变的自有元数据，因此满足字段所允许的 `Send + Sync`。应用通过 `Arc<MvtComposer>` 在多个线程或异步请求之间共享，无需加锁。每次 `compose()` 调用独立拥有自己的解压缓冲区、未压缩 Composite MVT 和最终压缩输出。

配置替换属于业务集成层职责。应用可以构建并完整校验新的 Composer，再使用 `arc-swap` 原子替换旧的 `Arc`。本库不强制依赖 `arc-swap`。

## 11. 测试与验收标准

### 11.1 Source 测试

- 显式创建未压缩 source；
- 解析未压缩、gzip、zstd 和 Brotli 样本；
- 自动识别 gzip 和 zstd；
- 显式解析 Brotli；
- 空字节与非法 MVT；
- `from_mvts()` 接收到空样本集合；
- 无图层、缺少图层名、空图层名和多个图层；
- 单个样本中的重复图层名；
- 多样本图层并集及第一次出现顺序；
- 自动识别样本的压缩格式不一致；
- 各解码器 feature 未启用；
- `Compression::None` 返回 borrowed bytes；
- 压缩格式返回 owned 解压结果。

### 11.2 Builder 测试

- 无 source；
- 重复 source ID；
- source 不含图层；
- 空图层名；
- 同一 source 内部图层重名；
- 不同 source 图层重名时的 `Allow` 和 `Error`；
- 独立调用 `validate_duplicate_layers()` 与 `build()` 得到一致结果；
- 压缩 feature 未启用；
- `Compression::Other`；
- 正常构建并保持 source 顺序。

### 11.3 Compose 测试

- 单 source 和多 source；
- 输入数量不足和过多；
- 空的原始 MVT bytes；
- 保持 source 顺序；
- 全部输入未压缩；
- 混合未压缩、gzip、zstd 和 Brotli 输入；
- 解压错误包含对应 source ID；
- 默认输出为未压缩格式；
- 显式 `Compression::None` 不产生第二次输出复制；
- gzip、zstd 和 Brotli 输出的格式签名与解压往返；
- 输出压缩失败包含输出格式上下文；
- 输出 feature 未启用时 Builder 构建失败；
- `Compression::Other` 不能作为输出格式；
- 输出相互独立且不修改输入；
- 通过可单独测试的 checked-length helper 覆盖长度溢出，因为实际申请 `usize::MAX` 字节不可行；
- 多线程共享同一个带固定输出压缩配置的 `Arc<MvtComposer>`，结果相互独立。

### 11.4 端到端与 Web 兼容测试

测试使用 `fast-mvt` writer 构建多个独立样本 MVT，按照 source 配置分别压缩，只调用一次公开的 `compose()`。当输出为 `None` 时直接使用 `fast-mvt` 读取结果；当输出为 gzip、zstd 或 Brotli 时先解压最终输出，再使用 `fast-mvt` 读取。全部预期图层必须存在，并保持固定顺序。

Web 兼容测试配置 `output_compression(Compression::Gzip)`，验证返回值只包含一个完整 gzip member，解压后再次验证全部图层。文档明确将“最终结果整体压缩”定义为受支持的 HTTP 交付方式，并记录各格式对应的 `Content-Encoding`。

### 11.5 发布前验证矩阵

发布前必须通过：

- `cargo fmt --check`；
- `cargo clippy --all-targets --all-features -- -D warnings`；
- 无默认 feature 的测试；
- 默认 gzip feature 的测试；
- 每个可选解码器单独启用时的测试；
- 全部 feature 同时启用时的测试；
- rustdoc 文档测试；
- release build；
- `cargo package` 或 `cargo publish --dry-run`，但不实际发布。

示例程序作为库的手工验收入口：创建使用混合输入压缩格式的 sources，执行组合，并通过 `fast-mvt` 证明返回值中的每个图层均可读取。

## 12. crates.io 发布质量要求

仓库必须包含：

- `README.md`，提供快速开始、feature 表、输入与输出压缩语义、并发示例、`Content-Encoding` 对应关系和 HTTP 交付警告；
- `LICENSE-MIT` 与 `LICENSE-APACHE`；
- 完整的 crate 元数据、categories、keywords、文档地址和 MSRV；
- 所有公开类型、方法和错误的 rustdoc；
- 从 `0.1.0` 开始的 changelog；
- 除非用户另行明确要求，否则实现任务不执行实际发布。

只有在存在真实公共仓库地址时才写入 repository 和 homepage 元数据；不得发布占位 URL。
