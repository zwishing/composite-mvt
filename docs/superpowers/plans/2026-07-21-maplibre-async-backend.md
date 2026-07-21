# MapLibre 异步后端 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 MapLibre 服务端示例迁移为完整的异步 HTTP 服务，并并发查询上游 MVT 数据源。

**Architecture:** 使用 Axum 承载全部现有路由，Tokio 负责运行时与异步锁，Reqwest 负责复用连接的上游请求。`AppState::compose_tile` 收集所有异步请求的结果后调用现有 `MvtComposer`，从而保持核心库 API 与输出语义不变。

**Tech Stack:** Rust 2024、Tokio、Axum、Reqwest（rustls）、futures-util、http-body-util、Tower。

## Global Constraints

- 保留 `PORT` 的默认值 `3010` 和所有既有 HTTP 路径、状态码、Content-Type。
- 不改变 `composite-mvt` 的公开 API 或 `src/` 下的核心合成逻辑。
- Reqwest 不启用自动 gzip 解压；压缩输入必须原样传给 `MvtComposer`。
- 上游请求超时保持 30 秒；多个已配置源必须并发查询。
- 每次代码编辑前运行 `gitnexus impact <symbol> --direction upstream --include-tests`，高风险结果须先报告。

---

### Task 1: 声明异步示例依赖

**Files:**
- Modify: `Cargo.toml:25-33`
- Modify: `Cargo.lock`

**Interfaces:**
- Produces: Tokio runtime、Axum router、Reqwest HTTP client，以及异步 router 测试支持。

- [ ] **Step 1: 写出依赖替换的预期失败测试**

在 `examples/maplibre_server.rs` 临时引用以下符号：

```rust
use axum::Router;
use reqwest::Client;
use tokio::sync::RwLock;

fn async_stack_is_available(_: Router, _: Client, _: RwLock<()>) {}
```

- [ ] **Step 2: 运行测试以确认依赖尚不可用**

Run: `cargo test --example maplibre_server --features gzip`

Expected: 编译失败，提示 `axum`、`reqwest` 和 `tokio` 未解析。

- [ ] **Step 3: 写入最小依赖声明并移除 tiny_http**

将 `Cargo.toml` 的开发依赖替换为：

```toml
[dev-dependencies]
axum = "0.8"
fast-mvt = { version = "=0.6.0", default-features = false, features = ["reader", "writer"] }
futures-util = "0.3"
http-body-util = "0.1"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "http2"] }
tokio = { version = "1", features = ["macros", "net", "rt-multi-thread", "sync", "time"] }
tower = { version = "0.5", features = ["util"] }
```

移除步骤 1 的临时函数；运行 `cargo check --example maplibre_server --features gzip` 让 Cargo.lock 记录解析结果。

- [ ] **Step 4: 验证依赖可用**

Run: `cargo check --example maplibre_server --features gzip`

Expected: exit code 0。

### Task 2: 将状态与上游查询改为异步并发

**Files:**
- Modify: `examples/maplibre_server/state.rs:1-128,271-322`
- Test: `examples/maplibre_server/tests.rs`

**Interfaces:**
- Consumes: `reqwest::Client`、`tokio::sync::RwLock`、`futures_util::future::try_join_all`。
- Produces: `AppState::configure(&self, &str) -> Result<(), String>` 和 `AppState::compose_tile(&self, &str, &str, &str) -> impl Future<Output = Result<Vec<u8>, String>>`。

- [ ] **Step 1: 写失败的异步配置/瓦片测试**

在测试模块加入：

```rust
#[tokio::test]
async fn tile_route_requires_configuration() {
    let response = response(Method::GET, "/tiles/0/0/0.pbf", &[], AppState::new()).await;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(body(response).await, b"configure sources first\n");
}
```

其中 `response` 和 `body` 在 Task 3 改为异步 Axum 测试帮助函数。

- [ ] **Step 2: 运行测试以确认旧同步接口不匹配**

Run: `cargo test --example maplibre_server tile_route_requires_configuration --features gzip`

Expected: 编译失败，原因是现有 router/response helper 不是 Future。

- [ ] **Step 3: 实现异步状态与并发请求**

将状态持有者定义为：

```rust
pub(crate) struct AppState {
    configured: RwLock<Option<ConfiguredSources>>,
    client: reqwest::Client,
}

impl AppState {
    pub(crate) fn new() -> Self {
        Self {
            configured: RwLock::new(None),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("default HTTP client configuration is valid"),
        }
    }

    pub(crate) async fn compose_tile(
        &self, z: &str, x: &str, y: &str,
    ) -> Result<Vec<u8>, String> {
        let configured = self.configured.read().await;
        let configured = configured.as_ref().ok_or("configure sources first")?;
        let requests = configured.sources.iter().map(|source| {
            let url = source.url.replace("{z}", z).replace("{x}", x).replace("{y}", y);
            fetch_http(&self.client, url, source.gzip)
        });
        let inputs = futures_util::future::try_join_all(requests).await?;
        let borrowed = inputs.iter().map(Vec::as_slice).collect::<Vec<_>>();
        configured.composer.compose(&borrowed)
            .map(|tile| tile.to_vec())
            .map_err(|error| error.to_string())
    }
}
```

将 `fetch_http` 改为：

```rust
async fn fetch_http(client: &reqwest::Client, url: String, gzip: bool) -> Result<Vec<u8>, String> {
    let accept_encoding = if gzip { "gzip" } else { "identity" };
    let response = client.get(&url)
        .header(reqwest::header::ACCEPT_ENCODING, accept_encoding)
        .send().await
        .map_err(|error| format!("failed to fetch {url}: {error}"))?
        .error_for_status()
        .map_err(|error| format!("failed to fetch {url}: {error}"))?;
    response.bytes().await
        .map(|bytes| bytes.to_vec())
        .map_err(|error| format!("failed to read {url}: {error}"))
}
```

`configure` 在写锁处改为 `self.configured.write().await` 并标记为 `async`；相应调用方必须 `.await`。

- [ ] **Step 4: 验证状态层行为**

Run: `cargo test --example maplibre_server tile_route_requires_configuration --features gzip`

Expected: 测试通过，未配置时仍返回 502 与原错误文本。

### Task 3: 用 Axum 迁移路由和服务启动

**Files:**
- Modify: `examples/maplibre_server/http.rs`
- Modify: `examples/maplibre_server/state.rs:271-300`
- Modify: `examples/maplibre_server.rs:12-15`

**Interfaces:**
- Consumes: `AppState::configure(...).await`、`AppState::compose_tile(...).await`。
- Produces: `http::router(Arc<AppState>) -> axum::Router` 与 `state::run_from_environment() -> impl Future<Output = Result<(), Box<dyn Error + Send + Sync>>>`。

- [ ] **Step 1: 写失败的健康检查 router 测试**

```rust
#[tokio::test]
async fn health_route_returns_ok() {
    let response = response(Method::GET, "/health", &[], AppState::new()).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await, b"ok\n");
}
```

- [ ] **Step 2: 运行测试确认 tiny_http 不能提供 Axum router**

Run: `cargo test --example maplibre_server health_route_returns_ok --features gzip`

Expected: 编译失败，`http::router` 未定义或返回类型不支持 `oneshot`。

- [ ] **Step 3: 实现 Axum fallback handler 与启动入口**

在 `http.rs` 定义以下接口：

```rust
pub(crate) fn router(state: Arc<AppState>) -> Router {
    Router::new().fallback(any(dispatch)).with_state(state)
}

async fn dispatch(
    State(state): State<Arc<AppState>>,
    method: Method,
    uri: Uri,
    body: Bytes,
) -> Response {
    let path = uri.path();
    // 保留 /sources、静态资源、/health、fixtures、/tiles 和 404 的现有匹配顺序与状态码。
}
```

其中 `POST /sources` 使用 `std::str::from_utf8(&body)` 后调用 `state.configure(body).await`；瓦片路径调用 `state.compose_tile(z, x, y).await`。所有二进制成功响应设置 `application/vnd.mapbox-vector-tile`，静态资源设置现有 Content-Type，瓦片额外设置 `cache-control: no-store`。

将启动逻辑替换为：

```rust
pub(crate) async fn run_from_environment() -> Result<(), Box<dyn Error + Send + Sync>> {
    let port = parse_port()?;
    let address = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("listening on http://{address}");
    axum::serve(listener, crate::http::router(Arc::new(AppState::new()))).await?;
    Ok(())
}
```

并将 example 的入口改为：

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    state::run_from_environment().await
}
```

- [ ] **Step 4: 验证 router 和服务编译**

Run: `cargo test --example maplibre_server health_route_returns_ok --features gzip`

Expected: 测试通过。

### Task 4: 迁移完整路由测试并证明并发请求

**Files:**
- Modify: `examples/maplibre_server/tests.rs`
- Modify: `README.md:112-128`

**Interfaces:**
- Consumes: `http::router(Arc<AppState>)`。
- Produces: 以 `#[tokio::test]` 覆盖 router 语义，以及受控本地上游的并发查询回归测试。

- [ ] **Step 1: 将测试帮助函数改为 Axum 版本**

```rust
async fn response(method: Method, path: &str, body: &[u8], state: AppState) -> Response {
    router(Arc::new(state)).oneshot(
        Request::builder().method(method).uri(path).body(Body::from(body.to_vec())).unwrap(),
    ).await.unwrap()
}

async fn body(response: Response) -> Vec<u8> {
    response.into_body().collect().await.unwrap().to_bytes().to_vec()
}
```

将每个 `#[test]` 改为 `#[tokio::test]`，并在所有 `response(...)` 与 `body(...)` 调用处使用 `.await`。断言改用 `StatusCode::{OK,BAD_REQUEST,METHOD_NOT_ALLOWED,NOT_FOUND,BAD_GATEWAY}`。

- [ ] **Step 2: 添加并发上游回归测试**

使用 Axum 在 `127.0.0.1:0` 启动测试上游；每个 `/tile/{id}` handler 先增加一个 `AtomicUsize` 计数，在两个请求都进入后通过 `tokio::sync::Notify` 同时放行，并返回同一份 fixture 字节。配置两个 URL 后请求 `/tiles/0/0/0.pbf`，断言状态为 200 且进入计数为 2。该断言会在顺序查询实现上超时，证明两个请求是同时发起的。

- [ ] **Step 3: 更新 README 实现说明**

将“uses the system `curl` command”替换为“uses an asynchronous Reqwest client and concurrently fetches configured tile sources”；保留离线 fixture 和 MapLibre GL 版本说明。

- [ ] **Step 4: 运行示例测试与完整测试套件**

Run: `cargo test --example maplibre_server --features gzip`

Expected: 全部 MapLibre 示例测试通过。

Run: `cargo test --all-features`

Expected: 全部项目测试通过。

### Task 5: 审核影响范围并提交

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `examples/maplibre_server.rs`
- Modify: `examples/maplibre_server/http.rs`
- Modify: `examples/maplibre_server/state.rs`
- Modify: `examples/maplibre_server/tests.rs`
- Modify: `README.md`

- [ ] **Step 1: 检查格式与差异**

Run: `cargo fmt --check && git diff --check && git diff --stat`

Expected: 三个命令均成功，差异只涉及上述文件与设计/计划文档。

- [ ] **Step 2: 运行 GitNexus 变更检测**

Run: `gitnexus detect_changes --scope compare --base-ref main`

Expected: 受影响流程仅包含 MapLibre 示例配置、路由和瓦片合成；不应出现 `src/` 核心库符号。

- [ ] **Step 3: 提交已验证变更**

```bash
git add Cargo.toml Cargo.lock README.md examples/maplibre_server.rs examples/maplibre_server docs/superpowers/specs/2026-07-21-maplibre-async-backend-design.md docs/superpowers/plans/2026-07-21-maplibre-async-backend.md
git commit -m "feat: make MapLibre example backend async"
```

