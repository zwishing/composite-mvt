use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{Method, Request, StatusCode},
    response::Response,
    routing::get,
};
use fast_mvt::MvtReaderRef;
use http_body_util::BodyExt as _;
use tokio::sync::Notify;
use tower::ServiceExt as _;

use crate::http::router;
use crate::state::{self, AppState};

async fn response(method: Method, path: &str, body: &[u8], state: AppState) -> Response {
    router(Arc::new(state))
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .body(Body::from(body.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap()
}

fn header_value<'a>(response: &'a Response, name: &'static str) -> Option<&'a str> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
}

async fn body(response: Response) -> Vec<u8> {
    response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec()
}

#[tokio::test]
async fn serves_the_single_html_frontend_and_local_maplibre_assets() {
    for (path, content_type) in [
        ("/", "text/html; charset=utf-8"),
        ("/maplibre-gl.js", "application/javascript; charset=utf-8"),
        ("/maplibre-gl.css", "text/css; charset=utf-8"),
    ] {
        let response = response(Method::GET, path, &[], AppState::new()).await;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_eq!(header_value(&response, "content-type"), Some(content_type));
    }
    let html =
        String::from_utf8(body(response(Method::GET, "/", &[], AppState::new()).await).await)
            .unwrap();
    assert!(html.contains("id=\"sources\""));
    assert!(!html.contains("/app.js"));
    assert!(html.contains("/fixtures/demo/{z}/{x}/{y}.pbf"));
    assert!(html.contains("/fixtures/open/{z}/{x}/{y}.pbf"));
}

#[tokio::test]
async fn accepts_multiple_source_descriptions() {
    let config = b"https://demotiles.maplibre.org/tiles/{z}/{x}/{y}.pbf\tgeolines,centroids,countries\tnone\nhttps://tiles.openfreemap.org/planet/latest/{z}/{x}/{y}.pbf\tlanduse\tnone";
    let response = response(Method::POST, "/sources", config, AppState::new()).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await, b"sources configured\n");
}

#[tokio::test]
async fn accepts_browser_encoded_template_placeholders() {
    let config = b"http://127.0.0.1:3010/a/%7Bz%7D/%7Bx%7D/%7By%7D.pbf\troads\tnone";
    assert_eq!(
        response(Method::POST, "/sources", config, AppState::new())
            .await
            .status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn rejects_incomplete_source_descriptions() {
    for config in [
        "",
        "ftp://example.com/{z}/{x}/{y}.pbf\troads\tnone",
        "http://example.com/tile.pbf\troads\tnone",
        "http://example.com/{z}/{x}/{y}.pbf\t\tnone",
        "http://example.com/{z}/{x}/{y}.pbf\troads\tbr",
    ] {
        assert_eq!(
            response(Method::POST, "/sources", config.as_bytes(), AppState::new())
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
}

#[tokio::test]
async fn local_fixtures_keep_the_expected_layers() {
    for (path, expected_layers) in [
        ("/fixtures/roads/0/0/0.pbf", &["roads"][..]),
        ("/fixtures/buildings/0/0/0.pbf", &["buildings"][..]),
        (
            "/fixtures/demo/1/0/0.pbf",
            &["geolines", "centroids", "countries"][..],
        ),
        (
            "/fixtures/demo/1/0/1.pbf",
            &["geolines", "centroids", "countries"][..],
        ),
        (
            "/fixtures/demo/1/1/0.pbf",
            &["geolines", "centroids", "countries"][..],
        ),
        (
            "/fixtures/demo/1/1/1.pbf",
            &["geolines", "centroids", "countries"][..],
        ),
        ("/fixtures/open/1/0/0.pbf", &[]),
        ("/fixtures/open/1/0/1.pbf", &[]),
        ("/fixtures/open/1/1/0.pbf", &[]),
        ("/fixtures/open/1/1/1.pbf", &[]),
        ("/fixtures/demo/2/2/1.pbf", &[]),
        ("/fixtures/open/2/2/1.pbf", &[]),
        ("/fixtures/demo/3/4/3.pbf", &[]),
        ("/fixtures/open/3/4/3.pbf", &[]),
        (
            "/fixtures/demo/4/8/5.pbf",
            &["geolines", "centroids", "countries"][..],
        ),
        ("/fixtures/open/4/8/5.pbf", &["landuse"][..]),
        ("/fixtures/demo/5/16/14.pbf", &[]),
        ("/fixtures/open/5/16/14.pbf", &[]),
    ] {
        let response = response(Method::GET, path, &[], AppState::new()).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            header_value(&response, "content-type"),
            Some("application/vnd.mapbox-vector-tile")
        );
        let tile = body(response).await;
        let reader = MvtReaderRef::new(&tile).unwrap();
        let layers = reader
            .layers()
            .map(|layer| layer.name())
            .collect::<Vec<_>>();
        for expected_layer in expected_layers {
            assert!(layers.contains(expected_layer), "{path}: {expected_layer}");
        }
    }
}

#[tokio::test]
async fn tile_route_requires_configuration() {
    let response = response(Method::GET, "/tiles/0/0/0.pbf", &[], AppState::new()).await;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(body(response).await, b"configure sources first\n");
}

#[tokio::test]
async fn health_route_returns_ok() {
    let response = response(Method::GET, "/health", &[], AppState::new()).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await, b"ok\n");
}

#[tokio::test]
async fn other_routes_and_methods_are_rejected() {
    assert_eq!(
        response(Method::GET, "/missing", &[], AppState::new())
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        response(Method::PUT, "/sources", &[], AppState::new())
            .await
            .status(),
        StatusCode::METHOD_NOT_ALLOWED
    );
    assert_eq!(state::DEFAULT_PORT, 3010);
}

struct UpstreamState {
    entered: AtomicUsize,
    ready: Notify,
}

async fn upstream_tile(
    State(state): State<Arc<UpstreamState>>,
    Path(path): Path<String>,
) -> Vec<u8> {
    if state.entered.fetch_add(1, Ordering::SeqCst) == 0 {
        tokio::time::timeout(Duration::from_millis(500), state.ready.notified())
            .await
            .expect("a sequential upstream request must not wait indefinitely");
    } else {
        state.ready.notify_waiters();
    }
    crate::state::fixture(if path.starts_with("a/") {
        "/fixtures/roads/0/0/0.pbf"
    } else {
        "/fixtures/buildings/0/0/0.pbf"
    })
    .unwrap()
    .to_vec()
}

#[tokio::test]
async fn tile_sources_are_fetched_concurrently() {
    let sequential_state = Arc::new(UpstreamState {
        entered: AtomicUsize::new(0),
        ready: Notify::new(),
    });
    let sequential_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let sequential_address = sequential_listener.local_addr().unwrap();
    let sequential_server_state = sequential_state.clone();
    let sequential_server = tokio::spawn(async move {
        axum::serve(
            sequential_listener,
            Router::new()
                .route("/tile/{*path}", get(upstream_tile))
                .with_state(sequential_server_state),
        )
        .await
    });
    assert!(
        tokio::time::timeout(
            Duration::from_millis(100),
            reqwest::get(format!("http://{sequential_address}/tile/a/0/0/0.pbf")),
        )
        .await
        .is_err(),
        "a sequential upstream fetch unexpectedly completed"
    );
    assert_eq!(sequential_state.entered.load(Ordering::SeqCst), 1);
    sequential_server.abort();

    let upstream_state = Arc::new(UpstreamState {
        entered: AtomicUsize::new(0),
        ready: Notify::new(),
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server_state = upstream_state.clone();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new()
                .route("/tile/{*path}", get(upstream_tile))
                .with_state(server_state),
        )
        .await
    });
    let state = AppState::new();
    let config = format!(
        "http://{address}/tile/a/{{z}}/{{x}}/{{y}}.pbf\troads\tnone\nhttp://{address}/tile/b/{{z}}/{{x}}/{{y}}.pbf\tbuildings\tnone"
    );
    state.configure(&config).await.unwrap();
    let response = tokio::time::timeout(
        Duration::from_secs(2),
        response(Method::GET, "/tiles/0/0/0.pbf", &[], state),
    )
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(upstream_state.entered.load(Ordering::SeqCst), 2);
    let tile = body(response).await;
    let tile_reader = MvtReaderRef::new(&tile).unwrap();
    let layers = tile_reader
        .layers()
        .map(|layer| layer.name())
        .collect::<Vec<_>>();
    assert!(layers.contains(&"roads"));
    assert!(layers.contains(&"buildings"));
    server.abort();
}
