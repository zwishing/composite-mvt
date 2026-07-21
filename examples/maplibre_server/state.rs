use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use composite_mvt::{Compression, MvtComposer, MvtSource};
use futures_util::future::try_join_all;
use tokio::sync::RwLock;

pub(crate) const DEFAULT_PORT: u16 = 3010;

struct SourceConfig {
    url: String,
    gzip: bool,
}

struct ConfiguredSources {
    sources: Vec<SourceConfig>,
    composer: MvtComposer,
}

pub(crate) struct AppState {
    configured: RwLock<Option<ConfiguredSources>>,
    client: reqwest::Client,
}

impl AppState {
    pub(crate) fn new() -> Self {
        Self {
            configured: RwLock::new(None),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("default HTTP client configuration is valid"),
        }
    }

    pub(crate) async fn configure(&self, body: &str) -> Result<(), String> {
        let mut builder = MvtComposer::builder();
        let mut sources = Vec::new();

        for (index, line) in body
            .lines()
            .filter(|line| !line.trim().is_empty())
            .enumerate()
        {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 3 {
                return Err(format!(
                    "source {} must contain URL, layers, and compression",
                    index + 1
                ));
            }

            let url = fields[0]
                .trim()
                .replace("%7B", "{")
                .replace("%7b", "{")
                .replace("%7D", "}")
                .replace("%7d", "}");
            if !(url.starts_with("http://") || url.starts_with("https://"))
                || !url.contains("{z}")
                || !url.contains("{x}")
                || !url.contains("{y}")
            {
                return Err(format!(
                    "source {} needs an HTTP(S) URL with {{z}}, {{x}}, and {{y}}: {url}",
                    index + 1,
                ));
            }

            let layers = fields[1]
                .split(',')
                .map(str::trim)
                .filter(|layer| !layer.is_empty())
                .collect::<Vec<_>>();
            if layers.is_empty() {
                return Err(format!("source {} needs at least one layer", index + 1));
            }

            let compression = match fields[2].trim() {
                "none" => Compression::None,
                "gzip" => Compression::Gzip,
                _ => {
                    return Err(format!(
                        "source {} compression must be none or gzip",
                        index + 1
                    ));
                }
            };
            let source = MvtSource::new(format!("source-{}", index + 1))
                .with_layers(layers)
                .with_compression(compression);
            builder = builder.add_source(source);
            sources.push(SourceConfig {
                url,
                gzip: compression == Compression::Gzip,
            });
        }

        if sources.is_empty() {
            return Err("add at least one source".to_owned());
        }

        let configured = ConfiguredSources {
            sources,
            composer: builder.build().map_err(|error| error.to_string())?,
        };
        *self.configured.write().await = Some(configured);
        Ok(())
    }

    pub(crate) async fn compose_tile(&self, z: &str, x: &str, y: &str) -> Result<Vec<u8>, String> {
        let configured = self.configured.read().await;
        let configured = configured.as_ref().ok_or("configure sources first")?;
        let requests = configured.sources.iter().map(|source| {
            let url = source
                .url
                .replace("{z}", z)
                .replace("{x}", x)
                .replace("{y}", y);
            fetch_http(&self.client, url, source.gzip)
        });
        let inputs = try_join_all(requests).await?;
        let borrowed = inputs.iter().map(Vec::as_slice).collect::<Vec<_>>();
        configured
            .composer
            .compose(&borrowed)
            .map(|tile| tile.to_vec())
            .map_err(|error| error.to_string())
    }
}

pub(crate) fn html() -> &'static [u8] {
    include_bytes!("../maplibre/index.html")
}

pub(crate) fn maplibre_js() -> &'static [u8] {
    include_bytes!("../maplibre/maplibre-gl.js")
}

pub(crate) fn maplibre_css() -> &'static [u8] {
    include_bytes!("../maplibre/styles/maplibre-gl.css")
}

pub(crate) fn fixture(path: &str) -> Option<&'static [u8]> {
    match path {
        "/fixtures/roads/0/0/0.pbf" => Some(include_bytes!("../maplibre/fixtures/roads.pbf")),
        "/fixtures/buildings/0/0/0.pbf" => {
            Some(include_bytes!("../maplibre/fixtures/buildings.pbf"))
        }
        "/fixtures/demo/1/0/0.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/1/0/0.pbf")),
        "/fixtures/demo/1/0/1.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/1/0/1.pbf")),
        "/fixtures/demo/1/1/0.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/1/1/0.pbf")),
        "/fixtures/demo/1/1/1.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/1/1/1.pbf")),
        "/fixtures/open/1/0/0.pbf" => Some(include_bytes!("../maplibre/fixtures/open/1/0/0.pbf")),
        "/fixtures/open/1/0/1.pbf" => Some(include_bytes!("../maplibre/fixtures/open/1/0/1.pbf")),
        "/fixtures/open/1/1/0.pbf" => Some(include_bytes!("../maplibre/fixtures/open/1/1/0.pbf")),
        "/fixtures/open/1/1/1.pbf" => Some(include_bytes!("../maplibre/fixtures/open/1/1/1.pbf")),
        "/fixtures/demo/4/7/4.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/4/7/4.pbf")),
        "/fixtures/demo/4/7/5.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/4/7/5.pbf")),
        "/fixtures/demo/4/7/6.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/4/7/6.pbf")),
        "/fixtures/demo/4/8/4.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/4/8/4.pbf")),
        "/fixtures/demo/4/8/5.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/4/8/5.pbf")),
        "/fixtures/demo/4/8/6.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/4/8/6.pbf")),
        "/fixtures/demo/4/9/4.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/4/9/4.pbf")),
        "/fixtures/demo/4/9/5.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/4/9/5.pbf")),
        "/fixtures/demo/4/9/6.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/4/9/6.pbf")),
        "/fixtures/open/4/7/4.pbf" => Some(include_bytes!("../maplibre/fixtures/open/4/7/4.pbf")),
        "/fixtures/open/4/7/5.pbf" => Some(include_bytes!("../maplibre/fixtures/open/4/7/5.pbf")),
        "/fixtures/open/4/7/6.pbf" => Some(include_bytes!("../maplibre/fixtures/open/4/7/6.pbf")),
        "/fixtures/open/4/8/4.pbf" => Some(include_bytes!("../maplibre/fixtures/open/4/8/4.pbf")),
        "/fixtures/open/4/8/5.pbf" => Some(include_bytes!("../maplibre/fixtures/open/4/8/5.pbf")),
        "/fixtures/open/4/8/6.pbf" => Some(include_bytes!("../maplibre/fixtures/open/4/8/6.pbf")),
        "/fixtures/open/4/9/4.pbf" => Some(include_bytes!("../maplibre/fixtures/open/4/9/4.pbf")),
        "/fixtures/open/4/9/5.pbf" => Some(include_bytes!("../maplibre/fixtures/open/4/9/5.pbf")),
        "/fixtures/open/4/9/6.pbf" => Some(include_bytes!("../maplibre/fixtures/open/4/9/6.pbf")),
        "/fixtures/demo/2/1/0.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/2/1/0.pbf")),
        "/fixtures/demo/2/1/1.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/2/1/1.pbf")),
        "/fixtures/demo/2/1/2.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/2/1/2.pbf")),
        "/fixtures/demo/2/2/0.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/2/2/0.pbf")),
        "/fixtures/demo/2/2/1.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/2/2/1.pbf")),
        "/fixtures/demo/2/2/2.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/2/2/2.pbf")),
        "/fixtures/demo/2/3/0.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/2/3/0.pbf")),
        "/fixtures/demo/2/3/1.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/2/3/1.pbf")),
        "/fixtures/demo/2/3/2.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/2/3/2.pbf")),
        "/fixtures/open/2/1/0.pbf" => Some(include_bytes!("../maplibre/fixtures/open/2/1/0.pbf")),
        "/fixtures/open/2/1/1.pbf" => Some(include_bytes!("../maplibre/fixtures/open/2/1/1.pbf")),
        "/fixtures/open/2/1/2.pbf" => Some(include_bytes!("../maplibre/fixtures/open/2/1/2.pbf")),
        "/fixtures/open/2/2/0.pbf" => Some(include_bytes!("../maplibre/fixtures/open/2/2/0.pbf")),
        "/fixtures/open/2/2/1.pbf" => Some(include_bytes!("../maplibre/fixtures/open/2/2/1.pbf")),
        "/fixtures/open/2/2/2.pbf" => Some(include_bytes!("../maplibre/fixtures/open/2/2/2.pbf")),
        "/fixtures/open/2/3/0.pbf" => Some(include_bytes!("../maplibre/fixtures/open/2/3/0.pbf")),
        "/fixtures/open/2/3/1.pbf" => Some(include_bytes!("../maplibre/fixtures/open/2/3/1.pbf")),
        "/fixtures/open/2/3/2.pbf" => Some(include_bytes!("../maplibre/fixtures/open/2/3/2.pbf")),
        "/fixtures/demo/3/3/2.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/3/3/2.pbf")),
        "/fixtures/demo/3/3/3.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/3/3/3.pbf")),
        "/fixtures/demo/3/3/4.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/3/3/4.pbf")),
        "/fixtures/demo/3/4/2.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/3/4/2.pbf")),
        "/fixtures/demo/3/4/3.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/3/4/3.pbf")),
        "/fixtures/demo/3/4/4.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/3/4/4.pbf")),
        "/fixtures/demo/3/5/2.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/3/5/2.pbf")),
        "/fixtures/demo/3/5/3.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/3/5/3.pbf")),
        "/fixtures/demo/3/5/4.pbf" => Some(include_bytes!("../maplibre/fixtures/demo/3/5/4.pbf")),
        "/fixtures/open/3/3/2.pbf" => Some(include_bytes!("../maplibre/fixtures/open/3/3/2.pbf")),
        "/fixtures/open/3/3/3.pbf" => Some(include_bytes!("../maplibre/fixtures/open/3/3/3.pbf")),
        "/fixtures/open/3/3/4.pbf" => Some(include_bytes!("../maplibre/fixtures/open/3/3/4.pbf")),
        "/fixtures/open/3/4/2.pbf" => Some(include_bytes!("../maplibre/fixtures/open/3/4/2.pbf")),
        "/fixtures/open/3/4/3.pbf" => Some(include_bytes!("../maplibre/fixtures/open/3/4/3.pbf")),
        "/fixtures/open/3/4/4.pbf" => Some(include_bytes!("../maplibre/fixtures/open/3/4/4.pbf")),
        "/fixtures/open/3/5/2.pbf" => Some(include_bytes!("../maplibre/fixtures/open/3/5/2.pbf")),
        "/fixtures/open/3/5/3.pbf" => Some(include_bytes!("../maplibre/fixtures/open/3/5/3.pbf")),
        "/fixtures/open/3/5/4.pbf" => Some(include_bytes!("../maplibre/fixtures/open/3/5/4.pbf")),
        "/fixtures/demo/5/15/13.pbf" => {
            Some(include_bytes!("../maplibre/fixtures/demo/5/15/13.pbf"))
        }
        "/fixtures/demo/5/15/14.pbf" => {
            Some(include_bytes!("../maplibre/fixtures/demo/5/15/14.pbf"))
        }
        "/fixtures/demo/5/15/15.pbf" => {
            Some(include_bytes!("../maplibre/fixtures/demo/5/15/15.pbf"))
        }
        "/fixtures/demo/5/16/13.pbf" => {
            Some(include_bytes!("../maplibre/fixtures/demo/5/16/13.pbf"))
        }
        "/fixtures/demo/5/16/14.pbf" => {
            Some(include_bytes!("../maplibre/fixtures/demo/5/16/14.pbf"))
        }
        "/fixtures/demo/5/16/15.pbf" => {
            Some(include_bytes!("../maplibre/fixtures/demo/5/16/15.pbf"))
        }
        "/fixtures/demo/5/17/13.pbf" => {
            Some(include_bytes!("../maplibre/fixtures/demo/5/17/13.pbf"))
        }
        "/fixtures/demo/5/17/14.pbf" => {
            Some(include_bytes!("../maplibre/fixtures/demo/5/17/14.pbf"))
        }
        "/fixtures/demo/5/17/15.pbf" => {
            Some(include_bytes!("../maplibre/fixtures/demo/5/17/15.pbf"))
        }
        "/fixtures/open/5/15/13.pbf" => {
            Some(include_bytes!("../maplibre/fixtures/open/5/15/13.pbf"))
        }
        "/fixtures/open/5/15/14.pbf" => {
            Some(include_bytes!("../maplibre/fixtures/open/5/15/14.pbf"))
        }
        "/fixtures/open/5/15/15.pbf" => {
            Some(include_bytes!("../maplibre/fixtures/open/5/15/15.pbf"))
        }
        "/fixtures/open/5/16/13.pbf" => {
            Some(include_bytes!("../maplibre/fixtures/open/5/16/13.pbf"))
        }
        "/fixtures/open/5/16/14.pbf" => {
            Some(include_bytes!("../maplibre/fixtures/open/5/16/14.pbf"))
        }
        "/fixtures/open/5/16/15.pbf" => {
            Some(include_bytes!("../maplibre/fixtures/open/5/16/15.pbf"))
        }
        "/fixtures/open/5/17/13.pbf" => {
            Some(include_bytes!("../maplibre/fixtures/open/5/17/13.pbf"))
        }
        "/fixtures/open/5/17/14.pbf" => {
            Some(include_bytes!("../maplibre/fixtures/open/5/17/14.pbf"))
        }
        "/fixtures/open/5/17/15.pbf" => {
            Some(include_bytes!("../maplibre/fixtures/open/5/17/15.pbf"))
        }
        _ => None,
    }
}

pub(crate) async fn run_from_environment() -> Result<(), Box<dyn Error + Send + Sync>> {
    let port = match std::env::var("PORT") {
        Ok(value) => value
            .parse::<u16>()
            .map_err(|error| format!("failed to parse PORT={value}: {error}"))?,
        Err(std::env::VarError::NotPresent) => DEFAULT_PORT,
        Err(std::env::VarError::NotUnicode(_)) => return Err("PORT is not valid Unicode".into()),
    };

    let address = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("listening on http://{address}");
    axum::serve(listener, crate::http::router(Arc::new(AppState::new()))).await?;
    Ok(())
}

async fn fetch_http(client: &reqwest::Client, url: String, gzip: bool) -> Result<Vec<u8>, String> {
    let accept_encoding = if gzip { "gzip" } else { "identity" };
    let response = client
        .get(&url)
        .header(reqwest::header::ACCEPT_ENCODING, accept_encoding)
        .send()
        .await
        .map_err(|error| format!("failed to fetch {url}: {error}"))?
        .error_for_status()
        .map_err(|error| format!("failed to fetch {url}: {error}"))?;
    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|error| format!("failed to read {url}: {error}"))
}
