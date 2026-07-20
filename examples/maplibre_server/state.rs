use std::error::Error;
use std::process::Command;
use std::sync::RwLock;

use composite_mvt::{Compression, MvtComposer, MvtSource};

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
}

impl AppState {
    pub(crate) fn new() -> Self {
        Self {
            configured: RwLock::new(None),
        }
    }

    pub(crate) fn configure(&self, body: &str) -> Result<(), String> {
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
        *self
            .configured
            .write()
            .map_err(|_| "configuration lock is poisoned")? = Some(configured);
        Ok(())
    }

    pub(crate) fn compose_tile(&self, z: &str, x: &str, y: &str) -> Result<Vec<u8>, String> {
        let configured = self
            .configured
            .read()
            .map_err(|_| "configuration lock is poisoned")?;
        let configured = configured.as_ref().ok_or("configure sources first")?;
        let mut inputs = Vec::with_capacity(configured.sources.len());

        for source in &configured.sources {
            let url = source
                .url
                .replace("{z}", z)
                .replace("{x}", x)
                .replace("{y}", y);
            inputs.push(fetch_http(&url, source.gzip)?);
        }
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

pub(crate) fn run_from_environment() -> Result<(), Box<dyn Error + Send + Sync>> {
    let port = match std::env::var("PORT") {
        Ok(value) => value
            .parse::<u16>()
            .map_err(|error| format!("failed to parse PORT={value}: {error}"))?,
        Err(std::env::VarError::NotPresent) => DEFAULT_PORT,
        Err(std::env::VarError::NotUnicode(_)) => return Err("PORT is not valid Unicode".into()),
    };

    let state = std::sync::Arc::new(AppState::new());

    let address = format!("127.0.0.1:{port}");
    let server =
        tiny_http::Server::http(&address).map_err(|error| -> Box<dyn Error + Send + Sync> {
            format!("failed to bind {address}: {error}").into()
        })?;
    println!("listening on http://{address}");

    for request in server.incoming_requests() {
        let state = std::sync::Arc::clone(&state);
        std::thread::spawn(move || {
            if let Err(error) = crate::http::serve(request, &state) {
                eprintln!("request handling failed: {error}");
            }
        });
    }

    Ok(())
}

fn fetch_http(url: &str, gzip: bool) -> Result<Vec<u8>, String> {
    let accept_encoding = if gzip { "gzip" } else { "identity" };
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--max-time",
            "30",
            "--header",
            &format!("Accept-Encoding: {accept_encoding}"),
            url,
        ])
        .output()
        .map_err(|error| format!("failed to run curl: {error}"))?;

    if output.status.success() {
        Ok(output.stdout)
    } else {
        let message = String::from_utf8_lossy(&output.stderr);
        Err(format!("failed to fetch {url}: {}", message.trim()))
    }
}
