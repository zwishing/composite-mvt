use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct AssetPaths {
    pub(crate) html: PathBuf,
    pub(crate) app_js: PathBuf,
    pub(crate) maplibre_js: PathBuf,
    pub(crate) maplibre_css: PathBuf,
    pub(crate) roads: PathBuf,
    pub(crate) buildings: PathBuf,
}

pub(crate) fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub(crate) fn paths(root: &Path) -> AssetPaths {
    let examples = root.join("examples").join("maplibre");
    let maplibre_dist = root.join("node_modules").join("maplibre-gl").join("dist");

    AssetPaths {
        html: examples.join("index.html"),
        app_js: examples.join("app.js"),
        maplibre_js: maplibre_dist.join("maplibre-gl.js"),
        maplibre_css: maplibre_dist.join("maplibre-gl.css"),
        roads: examples.join("fixtures").join("roads.pbf"),
        buildings: examples.join("fixtures").join("buildings.pbf"),
    }
}

pub(crate) fn read_required(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path)
        .map_err(|error| format!("failed to read required file {}: {error}", path.display()))
}
