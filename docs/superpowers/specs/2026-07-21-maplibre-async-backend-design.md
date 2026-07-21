# MapLibre 示例：异步后端查询设计

## 目标

将 `maplibre_server` 示例从同步 `tiny_http`、每请求线程和外部 `curl` 调用，迁移为完整的 Tokio/Axum/Reqwest 异步栈。对外 HTTP 路由、配置格式、状态码和地图前端行为保持不变。

## 架构

- `run_from_environment` 在 Tokio 运行时中创建 Axum `Router` 并监听现有的 `PORT` 地址。
- `AppState` 由 Axum 状态共享；配置保存改用异步读写锁。
- 处理 `/tiles/{z}/{x}/{y}.pbf` 时，`compose_tile` 并发请求每个已配置上游数据源，收齐字节后同步调用 `MvtComposer` 合成结果。
- 复用单个 `reqwest::Client`，删除对系统 `curl` 的依赖。

## 压缩与错误处理

- 请求仍按每个源配置发送 `Accept-Encoding: gzip` 或 `identity`。
- Reqwest 不启用自动 gzip 解压，确保 gzip 源仍以压缩字节交给 `MvtComposer`，由其现有压缩处理逻辑解码。
- 配置错误继续返回 400；不支持的方法返回 405；未知路由返回 404；上游查询或合成错误返回 502，保留可读错误文本。

## 测试

- 将示例路由测试改为 Tokio 异步测试，直接调用 Axum router。
- 保留静态资源、配置校验、fixture 和未配置瓦片路由的覆盖。
- 新增回归测试：多个上游瓦片查询会并发执行，并能得到合成后的结果。

## 范围

仅修改 MapLibre 服务端示例及其依赖和测试；不改变库的公开 API、核心合成逻辑或浏览器前端协议。
