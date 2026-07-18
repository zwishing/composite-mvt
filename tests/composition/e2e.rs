use composite_mvt::{Compression, MvtComposer, MvtSource};

use super::{common, fixtures};

#[cfg(all(feature = "gzip", feature = "zstd"))]
#[test]
fn mixed_input_encodings_preserve_all_layers() {
    // Given: three real MVT inputs with distinct raw, gzip, and zstd encodings.
    let roads = common::tile_with_layers(&["roads"]);
    let pipeline = common::gzip(&common::tile_with_layers(&["pipeline", "valve"]));
    let buildings = fixtures::zstd_encode(&common::tile_with_layers(&["building"]));
    let composer = MvtComposer::builder()
        .add_source(MvtSource::new("roads").with_layers(["roads"]))
        .add_source(
            MvtSource::new("pipeline")
                .with_compression(Compression::Gzip)
                .with_layers(["pipeline", "valve"]),
        )
        .add_source(
            MvtSource::new("buildings")
                .with_compression(Compression::Zstd)
                .with_layers(["building"]),
        )
        .build()
        .unwrap();

    // When: the composer combines every source.
    let output = composer.compose(&[&roads, &pipeline, &buildings]).unwrap();

    // Then: a real MVT reader observes every layer in source order.
    assert_eq!(
        fixtures::layer_names(&output),
        ["roads", "pipeline", "valve", "building"]
    );
}

fn raw_two_layer_composer(output: Compression) -> MvtComposer {
    MvtComposer::builder()
        .output_compression(output)
        .add_source(MvtSource::new("roads").with_layers(["roads"]))
        .add_source(MvtSource::new("buildings").with_layers(["building"]))
        .build()
        .unwrap()
}

#[cfg(feature = "gzip")]
#[test]
fn emits_one_complete_gzip_output() {
    // Given: two raw MVT source tiles and gzip output.
    let roads = common::tile_with_layers(&["roads"]);
    let buildings = common::tile_with_layers(&["building"]);

    // When: the complete composite is compressed.
    let output = raw_two_layer_composer(Compression::Gzip)
        .compose(&[&roads, &buildings])
        .unwrap();

    // Then: one gzip stream decodes into a valid two-layer MVT.
    assert!(output.starts_with(&[0x1f, 0x8b]));
    assert_eq!(
        fixtures::layer_names(&fixtures::gunzip(&output)),
        ["roads", "building"]
    );
}

#[cfg(feature = "zstd")]
#[test]
fn emits_one_complete_zstd_output() {
    // Given: two raw MVT source tiles and zstd output.
    let roads = common::tile_with_layers(&["roads"]);
    let buildings = common::tile_with_layers(&["building"]);

    // When: the complete composite is compressed.
    let output = raw_two_layer_composer(Compression::Zstd)
        .compose(&[&roads, &buildings])
        .unwrap();

    // Then: one zstd stream decodes into a valid two-layer MVT.
    assert!(output.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]));
    assert_eq!(
        fixtures::layer_names(&fixtures::zstd_decode(&output)),
        ["roads", "building"]
    );
}

#[cfg(feature = "brotli")]
#[test]
fn emits_one_complete_brotli_output() {
    // Given: two raw MVT source tiles and brotli output.
    let roads = common::tile_with_layers(&["roads"]);
    let buildings = common::tile_with_layers(&["building"]);

    // When: the complete composite is compressed.
    let output = raw_two_layer_composer(Compression::Brotli)
        .compose(&[&roads, &buildings])
        .unwrap();

    // Then: one brotli stream decodes into a valid two-layer MVT.
    assert_eq!(
        fixtures::layer_names(&fixtures::brotli_decode(&output)),
        ["roads", "building"]
    );
}
