use std::error::Error;
use std::path::Path;

use composite_mvt::{Compression, MvtComposer, MvtSource};

use crate::assets;

pub(crate) struct AppState {
    pub(crate) composer: composite_mvt::MvtComposer,
    pub(crate) roads: Vec<u8>,
    pub(crate) buildings: Vec<u8>,
    pub(crate) html: Vec<u8>,
    pub(crate) app_js: Vec<u8>,
    pub(crate) maplibre_js: Vec<u8>,
    pub(crate) maplibre_css: Vec<u8>,
}

pub(crate) fn load(root: &Path) -> Result<AppState, Box<dyn Error + Send + Sync>> {
    let paths = assets::paths(root);

    let html = assets::read_required(&paths.html)
        .map_err(|error| -> Box<dyn Error + Send + Sync> { error.into() })?;
    let app_js = assets::read_required(&paths.app_js)
        .map_err(|error| -> Box<dyn Error + Send + Sync> { error.into() })?;
    let maplibre_js = assets::read_required(&paths.maplibre_js)
        .map_err(|error| -> Box<dyn Error + Send + Sync> { error.into() })?;
    let maplibre_css = assets::read_required(&paths.maplibre_css)
        .map_err(|error| -> Box<dyn Error + Send + Sync> { error.into() })?;

    let roads = assets::read_required(&paths.roads)
        .map_err(|error| -> Box<dyn Error + Send + Sync> { error.into() })?;
    let buildings = assets::read_required(&paths.buildings)
        .map_err(|error| -> Box<dyn Error + Send + Sync> { error.into() })?;

    let roads_source = MvtSource::from_mvt("roads", &roads)?;
    let buildings_source = MvtSource::from_mvt("buildings", &buildings)?;

    let composer = MvtComposer::builder()
        .output_compression(Compression::Gzip)
        .add_source(roads_source)
        .add_source(buildings_source)
        .build()?;

    Ok(AppState {
        composer,
        roads,
        buildings,
        html,
        app_js,
        maplibre_js,
        maplibre_css,
    })
}

pub(crate) fn run_from_environment() -> Result<(), Box<dyn Error + Send + Sync>> {
    let port = match std::env::var("PORT") {
        Ok(value) => value
            .parse::<u16>()
            .map_err(|error| format!("failed to parse PORT={value}: {error}"))?,
        Err(std::env::VarError::NotPresent) => 3000,
        Err(std::env::VarError::NotUnicode(_)) => return Err("PORT is not valid Unicode".into()),
    };

    let root = assets::repository_root();
    let state = load(&root)?;

    let address = format!("127.0.0.1:{port}");
    let server =
        tiny_http::Server::http(&address).map_err(|error| -> Box<dyn Error + Send + Sync> {
            format!("failed to bind {address}: {error}").into()
        })?;
    println!("listening on http://{address}");

    for request in server.incoming_requests() {
        if let Err(error) = crate::http::serve(request, &state) {
            return Err(format!("request handling failed: {error}").into());
        }
    }

    Ok(())
}

#[cfg(test)]
pub(crate) fn test_state() -> AppState {
    let root = assets::repository_root();
    load(&root).expect("load example assets")
}

#[cfg(test)]
pub(crate) fn test_state_with_invalid_composition_input() -> AppState {
    let mut state = test_state();
    let invalid_roads_source = MvtSource::new("roads")
        .with_layers(["roads"])
        .with_compression(Compression::Gzip);
    let buildings_source =
        MvtSource::from_mvt("buildings", &state.buildings).expect("valid buildings source");

    let composer = MvtComposer::builder()
        .output_compression(Compression::Gzip)
        .add_source(invalid_roads_source)
        .add_source(buildings_source)
        .build()
        .expect("invalid test composer");

    state.composer = composer;
    state
}
