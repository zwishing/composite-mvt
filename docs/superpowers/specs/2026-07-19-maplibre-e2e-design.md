# MapLibre 端到端瓦片合并测试设计

## 1. 目的

为 `composite-mvt` 增加一个可手动运行的 Rust 示例服务器，并用 MapLibre GL JS 与 Playwright 验证浏览器能够加载、解压、解析并查询合并后的 MVT。

测试必须覆盖完整链路：

1. server 读取两个固定 MVT 夹具；
2. `MvtComposer` 合并两个输入并对完整输出执行一次 gzip 压缩；
3. server 使用正确的 MVT 与 gzip HTTP 响应头返回瓦片；
4. MapLibre 请求并解析该瓦片；
5. Playwright 证明 `roads` 和 `buildings` 两个 source layer 都包含要素。

这套测试仅作为本地手动 E2E，不加入默认 `cargo test` 或 CI。

## 2. 已确认的约束

- server 是仓库中的公开示例程序，不是 crate 的公开 API；
- 使用仓库内固定夹具，不代理在线瓦片服务；
- 前端使用原生 HTML、CSS 和 JavaScript，不引入 Vite、React 或其他构建工具；
- MapLibre 与 Playwright 使用 npm 精确锁定版本，本地运行不依赖 CDN；
- server、页面、MapLibre 资源和瓦片使用同一 origin，不配置 CORS；
- MapLibre source 直接使用 `tiles` URL 模板，不实现 TileJSON；
- 浏览器测试通过 MapLibre 数据查询断言结果，不维护截图基线；
- 不改变 `composite-mvt` 的公开 API、错误类型或核心组合逻辑。

## 3. 总体架构

```text
roads.pbf ─────┐
               ├─ MvtComposer::compose() ─ gzip MVT ─┐
buildings.pbf ─┘                                      │
                                                      ▼
Playwright ── GET / ── MapLibre ── GET /tiles/0/0/0.pbf
                              │
                              └─ querySourceFeatures()
                                 - roads > 0
                                 - buildings > 0
```

### 3.1 Rust 示例服务器

`examples/maplibre_server.rs` 使用 `tiny_http 0.12.0` 实现同步 HTTP 服务。该 crate 只加入 `[dev-dependencies]`，不进入库使用者的依赖图。server 默认监听 `127.0.0.1:3000`，并允许通过 `PORT` 环境变量覆盖端口。

server 启动时：

1. 定位并读取两个固定夹具；
2. 创建包含 `roads` 与 `buildings` source 的 `MvtComposer`；
3. 将输出压缩固定为 `Compression::Gzip`；
4. 校验页面资源和本地 MapLibre 分发文件可读取；
5. 任一步失败时输出具体路径或错误链并以非零状态退出。

server 提供以下路由：

| Method | Path | Behavior |
|---|---|---|
| GET | `/health` | 返回 200，供 Playwright 等待 server 就绪 |
| GET | `/` | 返回 MapLibre 示例页面 |
| GET | `/app.js` | 返回页面逻辑 |
| GET | `/maplibre-gl.js` | 返回本地 npm 包中的 MapLibre 分发文件 |
| GET | `/maplibre-gl.css` | 返回本地 npm 包中的 MapLibre 样式 |
| GET | `/tiles/0/0/0.pbf` | 合并两个夹具并返回 gzip MVT |

未知路径返回 404，非 GET 请求返回 405。组合失败返回 500，不返回部分瓦片。

瓦片成功响应必须包含：

```text
Content-Type: application/vnd.mapbox-vector-tile
Content-Encoding: gzip
Cache-Control: no-store
```

### 3.2 固定 MVT 夹具

`examples/maplibre/fixtures/roads.pbf` 包含至少一个 line 类型要素，source layer 名为 `roads`。`examples/maplibre/fixtures/buildings.pbf` 包含至少一个 polygon 类型要素，source layer 名为 `buildings`。

两个要素必须位于 z0 瓦片的可视范围内，并使用 MapLibre 可以直接渲染的合法 MVT geometry。`examples/generate_maplibre_fixtures.rs` 使用现有 `fast-mvt` writer API 生成这两个文件：道路使用 `MvtGeometry::LineString`，建筑使用 `MvtGeometry::Polygon`，extent 固定为 4096，并给每个要素写入稳定的 ID 与 `kind` 属性。运行 `cargo run --example generate_maplibre_fixtures` 可确定性覆盖两个夹具。

夹具目录的 README 记录图层名、geometry 类型、坐标、属性、生成命令和预期 SHA-256，避免二进制文件成为无法审查的黑盒。生成器是开发工具，不被 server 在运行时调用。

server 只支持 z0 的 `0/0/0`，其他瓦片坐标返回 404。MapLibre source 同时设置 `minzoom: 0` 与 `maxzoom: 0`，确保只请求这一块瓦片。

### 3.3 MapLibre 页面

页面使用空白 style，不加载外部底图、glyph 或 sprite。它添加一个 vector source：

```javascript
{
  type: 'vector',
  tiles: ['/tiles/{z}/{x}/{y}.pbf'],
  minzoom: 0,
  maxzoom: 0
}
```

页面再添加两个 style layer：

- `roads-layer`：`type: 'line'`，`source-layer: 'roads'`；
- `buildings-layer`：`type: 'fill'`，`source-layer: 'buildings'`。

UI 只包含全屏地图和一个状态面板。状态面板使用机器可读的 `data-state`、`data-roads` 与 `data-buildings` 属性公开当前结果。

初始状态为 `loading`。MapLibre 进入 `idle` 后，页面分别调用 `querySourceFeatures` 查询 `roads` 与 `buildings`，写入数量，并在两个数量都大于零时切换为 `ready`。MapLibre `error` 事件将状态切换为 `error` 并显示错误消息。

## 4. Playwright 验收

根目录提供精确锁定版本的 npm 配置和 `playwright.config.ts`。`webServer` 使用以下语义启动示例：

```text
PORT=3100 cargo run --example maplibre_server --features gzip
```

E2E 测试执行以下断言：

1. `/health` 就绪后打开页面；
2. 捕获 `/tiles/0/0/0.pbf` 响应；
3. 断言状态码为 200；
4. 断言 `Content-Type` 为 MVT，`Content-Encoding` 为 gzip；
5. 等待状态面板进入 `ready`；
6. 断言 `data-roads` 和 `data-buildings` 都大于零；
7. 断言页面没有未处理的 `pageerror`。

测试不使用固定等待时间，不依赖截图，不访问公网，也不通过直接解析响应体替代 MapLibre 断言。

## 5. 文件布局

```text
examples/
├── generate_maplibre_fixtures.rs
├── maplibre_server.rs
└── maplibre/
    ├── index.html
    ├── app.js
    └── fixtures/
        ├── README.md
        ├── roads.pbf
        └── buildings.pbf
e2e/
└── maplibre.spec.ts
package.json
package-lock.json
playwright.config.ts
```

`node_modules/`、Playwright 浏览器与测试输出保持忽略，不进入 crate 运行时依赖。`tiny_http` 与 `fast-mvt` writer 仅用于示例、夹具生成和开发验证。

## 6. 本地运行

首次运行：

```bash
npm ci
npx playwright install chromium
```

手动查看：

```bash
cargo run --example maplibre_server --features gzip
```

然后打开 `http://127.0.0.1:3000`。

运行 E2E：

```bash
npm run test:e2e
```

## 7. 验证范围

实现完成后必须通过：

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo check --example maplibre_server --features gzip
npm run test:e2e
```

还需手动打开页面确认地图和状态面板可读。由于用户选择本地手动运行，本设计不修改 CI 工作流。

## 8. 非目标

- 不代理真实在线瓦片；
- 不实现 TileJSON、缓存、重试、鉴权或生产级 HTTP server；
- 不测试地图导航控件或业务交互；
- 不添加截图回归；
- 不发布 npm 包；
- 不把 Playwright E2E 加入默认 Rust 测试或 CI；
- 不修改 `MvtComposer` 的行为来适配示例。

## 9. 完成标准

- Rust 示例 server 可以独立启动并提供页面与合并瓦片；
- 返回的是一个完整 gzip MVT，HTTP 响应头正确；
- MapLibre 在浏览器中真实加载并解析该瓦片；
- MapLibre 能查询到 `roads` 和 `buildings` 两个 source layer 的要素；
- Playwright 在本地稳定验证完整链路；
- 文档提供可重复的安装、手动查看和 E2E 命令；
- 现有 Rust feature 测试和 lint 不回归。

## 10. 已确认事实（行级语义）
- **MapLibre 查询链路（v5.19.0）**
  - `Map#querySourceFeatures` 在 `src/ui/map.ts:2020` 直接代理到 `this.style.querySourceFeatures(sourceId, parameters)`；即前端只要 source 存在就可拿到 style 层聚合结果。
  - `Style#querySourceFeatures` 在 `src/style/style.ts:1641-1649` 通过 `sourceID` 查 `tileManager`。
    - 如果 source 不存在，返回 `[]`；
    - 否则调用 `querySourceFeatures(tileManager, params ? {..., globalState} : {globalState})`。
  - 底层实现 `querySourceFeatures`（`src/source/query_features.ts`）在可渲染 tile 列表中逐瓦片执行去重（`dataID`）后的 `tile.querySourceFeatures` 聚合，返回单一数组。
  - `Style.addSource` 在 `src/style/style.ts:1002-1007` 检测重复 id 时抛 `Error('Source "<id> already exists."')`；服务端页面应只添加一次 source。

- **tiny_http 语义（v0.12.0）**
  - `Request` 暴露 `method()` 与 `url()`（`src/request.rs`）用于路由分支判断。
  - `Method` 枚举包含 `Get`，可与 `req.method()` 对比实现 GET/非 GET 405 分流（`src/common.rs:252-259`）。
  - `StatusCode` 的原因表显式覆盖 `404 Not Found` 与 `405 Method Not Allowed`、`500 Internal Server Error`、`200 OK` 等标准码（`src/common.rs:44-46, 68`）。
  - `Response` 提供常用构造器与链式设置：`from_string`（`src/response.rs:523-539`）、`from_data`（`src/response.rs:507-521`）、`with_status_code`（`src/response.rs:301-309`）、`with_header`（`src/response.rs:288-299`）。
  - `Response` 在 `src/response.rs:257-263` 明确忽略 `Connection`、`Trailer`、`Transfer-Encoding`、`Upgrade` 等禁止覆写头；`Content-Length` 与 `Content-Type` 有对应的覆盖/计算处理逻辑（`src/response.rs:266-283`）。
  - 通过 `with_header` 可显式注入 `application/vnd.mapbox-vector-tile` 与 `Content-Encoding: gzip`。

- **验收映射（以设计要求为准）**
  - 服务器路由应返回 200：`GET /health`、`GET /`、`GET /app.js`、`GET /maplibre-gl.js`、`GET /maplibre-gl.css`、`GET /tiles/0/0/0.pbf`；其他 path 404。
  - `/tiles/0/0/0.pbf` 之外的 tile 坐标返回 404；非 GET 请求先按已知路径返回 405（其余仍以 404 结束）。
  - `/tiles/0/0/0.pbf` 成功响应应携带 `Content-Type: application/vnd.mapbox-vector-tile` 与 `Content-Encoding: gzip`，并在浏览器端通过 `querySourceFeatures` 仅观察到 `roads`、`buildings` 两个 layer。
