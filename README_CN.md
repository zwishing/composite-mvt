# composite-mvt

[English](README.md)

`composite-mvt` 将一组固定的 Mapbox Vector Tile（MVT）源合成为一个响应体。
它是刻意保持字节级处理的合成器：请求合成时不会解析、合并、重命名或校验图层；在启动阶段配置一次源元数据后，每个请求只需按顺序提供每个源的一段字节。

## 快速开始

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

`add_source` 的调用顺序固定了源顺序。`compose(&inputs)` 必须接收相同数量的输入，且 `inputs[n]` 始终对应第 `n` 个源。

## 压缩模型

输入压缩格式由源固定。每个 `MvtSource` 声明其请求字节所需的格式；`compose` 会先解码所有被配置为压缩的输入，内部合并仅按源顺序拼接原始 MVT 字节。原始输入在此阶段会被借用，压缩输入则需要分配已解码的缓冲区。

随后，合成器要么原样返回合并后的原始 MVT，要么对**完整**合成结果应用一次固定的输出压缩。它绝不会为每个源分别输出压缩流。编码器参数不是公开 API；启用的编解码器使用默认参数，且每次请求不能覆盖输出格式。

对于源元数据，`MvtSource::from_mvt` 与 `from_mvts` 只会自动识别 gzip 和 Zstandard 帧；其他样本按原始 MVT 处理。Brotli 没有可靠的帧签名，必须通过 `from_mvt_with_compression` 或 `from_mvts_with_compression` 显式传入 `Compression::Brotli`。

通过 HTTP 返回压缩字节时，应使用 `composer.output_compression().content_encoding()` 设置响应头：

| 输出 | `Content-Encoding` |
| --- | --- |
| `Compression::None` | 不设置该响应头 |
| `Compression::Gzip` | `gzip` |
| `Compression::Zstd` | `zstd` |
| `Compression::Brotli` | `br` |

Gzip 输出是包裹完整合成 MVT 的单个 gzip member。此 crate 只创建字节，不创建 HTTP 响应；响应头和缓存策略由调用方负责。

## 功能特性

| 功能 | 默认启用 | 作用 |
| --- | --- | --- |
| `gzip` | 是 | 启用 gzip 源解码与完整输出压缩。 |
| `zstd` | 否 | 启用 Zstandard 源解码与完整输出压缩。 |
| `brotli` | 否 | 启用显式配置的 Brotli 源解码与完整输出压缩。 |

构建合成器时，选择未启用的编解码器会被拒绝。`Compression::Other` 是不受支持的标记，不能作为有效的源或输出压缩格式。

## 校验与错误

`MvtComposerBuilder::validate_duplicate_layers()` 可独立调用，不会消费或修改 builder。同一源内重复的图层始终会被拒绝。不同源之间，默认的 `DuplicateLayer::Error` 会拒绝重复；`DuplicateLayer::Allow` 会接受并保留重复。选择后者可能产生不符合规范的输出，因为 MVT 2.1 要求同一 tile 内的图层名按字节完全唯一。参见 [MVT 2.1 规范](https://github.com/mapbox/vector-tile-spec/blob/master/2.1/README.md#41-layers)。

`build()` 会执行相同的重复图层校验，以及源 ID、图层和功能特性校验。样本构造与源解码返回 `SourceError`；配置返回 `BuildError`；请求合成返回 `ComposeError`。解压失败会指出出错的已配置源。解析样本时，缺失图层名和显式空图层名都会返回 `SourceError::MissingLayerName`；显式配置的空名称仍会返回 `BuildError::EmptyLayerName`。合成失败不会返回部分合成结果。

## 并发与内存

`build()` 完成后，`MvtComposer` 不可变，且不包含互斥锁、缓存或请求级可变状态。可将其放入 `Arc<MvtComposer>` 并在线程间共享，无需库管理的锁。每次调用分别拥有已解码的源缓冲区、一个原始合成缓冲区，以及在选择压缩时额外的最终编码缓冲区。压缩期间会短暂同时持有原始和编码后的完整合成结果；这是首个发布版本为避免流式输出而作出的有意内存权衡。

## 示例

运行混合输入压缩示例；默认启用 `gzip`：

```text
cargo run --example mixed_sources
```

它会将原始和 gzip 输入合成，再把完整输出压缩为 gzip，并打印：

```text
compression=gzip
layers=roads,pipeline,valve,building
```

## MapLibre 浏览器示例

浏览器示例是单个纯 HTML 页面。可以添加任意数量的矢量瓦片 URL 模板、源图层名、输入压缩格式、渲染类型和颜色。Rust 后端保存这些源配置，获取对应瓦片，使用 `MvtComposer` 合成后返回一个 MVT。

运行示例：

```text
cargo run --example maplibre_server --features gzip
```

打开 `http://127.0.0.1:3010`，添加或删除瓦片源后，选择 **应用并显示**。每一行对应一个 MVT 源，并接受逗号分隔的源图层名；点、线、面要素会自动应用样式。默认配置会在 z=2 至 z=5 合并随包提供的 MapLibre 欧洲演示瓦片（包含 `geolines`、`centroids`、`countries`）和 OpenFreeMap 瓦片（包含 `landuse`），因此这些缩放级别可离线运行。示例使用异步 Reqwest 客户端并发获取已配置的瓦片源，并在本地包含 MapLibre GL JS 5.24.0。

## 测试

```text
cargo test --all-targets --all-features
```

## 许可证

本项目可按你的选择，使用 [Apache License, Version 2.0](LICENSE-APACHE) 或 [MIT License](LICENSE-MIT) 授权。