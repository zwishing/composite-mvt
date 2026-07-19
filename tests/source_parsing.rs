mod common;

use common::tile_with_layers;
use composite_mvt::{BuildError, Compression, DuplicateLayer, MvtComposer, MvtSource, SourceError};

const TILE_WITHOUT_LAYERS: &[u8] = &[0x08, 0x00];
const TILE_WITH_EMPTY_LAYER_NAME: &[u8] = &[0x1a, 0x04, 0x0a, 0x00, 0x78, 0x02];

#[test]
fn parses_uncompressed_layers_in_order() {
    let bytes = tile_with_layers(&["pipeline", "valve"]);
    let source = MvtSource::from_mvt("network", &bytes).unwrap();

    assert_eq!(source.id().as_ref(), "network");
    assert_eq!(source.compression(), Compression::None);
    assert_eq!(
        source
            .layers()
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>(),
        ["pipeline", "valve"]
    );
}

#[test]
fn unions_multiple_samples_in_first_seen_order() {
    let first = tile_with_layers(&["pipeline", "valve"]);
    let second = tile_with_layers(&["pipeline", "station"]);
    let source = MvtSource::from_mvts("network", [&first, &second]).unwrap();

    assert_eq!(
        source
            .layers()
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>(),
        ["pipeline", "valve", "station"]
    );
}

#[test]
fn rejects_empty_input_and_empty_sample_set() {
    assert!(matches!(
        MvtSource::from_mvt("empty", &[]),
        Err(SourceError::EmptyBytes)
    ));
    assert!(matches!(
        MvtSource::from_mvts::<Vec<&[u8]>, &[u8]>("empty", Vec::new()),
        Err(SourceError::NoSamples)
    ));
}

#[test]
fn rejects_a_nonempty_tile_without_layers() {
    // Given: valid protobuf bytes containing no MVT layer fields.
    // When: public source metadata parsing is requested.
    let result = MvtSource::from_mvt("empty", TILE_WITHOUT_LAYERS);

    // Then: the source reports a zero-layer tile, not empty input.
    assert!(matches!(result, Err(SourceError::NoLayers)));
}

#[test]
fn maps_an_explicit_empty_layer_name_to_missing_layer_name() {
    // Given: a valid version-2 MVT layer whose encoded name is explicitly empty.
    // When: public source metadata parsing is requested.
    let result = MvtSource::from_mvt("empty-name", TILE_WITH_EMPTY_LAYER_NAME);

    // Then: fast-mvt's single missing-or-empty name contract is preserved.
    assert!(matches!(result, Err(SourceError::MissingLayerName)));
}

#[test]
fn preserves_duplicate_layer_names_from_one_sample_for_builder_validation() {
    // Given: one real sample that declares the same layer twice.
    let bytes = tile_with_layers(&["roads", "roads"]);

    // When: source metadata is parsed directly.
    let source = MvtSource::from_mvt("roads", &bytes).unwrap();

    // Then: parsing preserves declarations for the builder's duplicate policy.
    assert_eq!(
        source
            .layers()
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>(),
        ["roads", "roads"]
    );
}

#[test]
fn multi_sample_parsing_preserves_duplicates_declared_inside_one_sample() {
    let duplicate = tile_with_layers(&["roads", "roads"]);
    let additional = tile_with_layers(&["buildings"]);

    let source = MvtSource::from_mvts("mixed", [&duplicate, &additional]).unwrap();

    assert_eq!(
        source
            .layers()
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>(),
        ["roads", "roads", "buildings"]
    );

    let result = MvtComposer::builder()
        .duplicate_layer(DuplicateLayer::Allow)
        .add_source(source)
        .build();
    assert!(matches!(
        result,
        Err(BuildError::DuplicateLayerName {
            layer,
            first_source,
            second_source,
        }) if layer.as_ref() == "roads"
            && first_source.as_ref() == "mixed"
            && second_source.as_ref() == "mixed"
    ));
}

#[test]
fn explicit_multi_sample_parsing_preserves_later_sample_duplicates() {
    let first = tile_with_layers(&["roads"]);
    let duplicate = tile_with_layers(&["roads", "roads"]);

    let source =
        MvtSource::from_mvts_with_compression("mixed", [&first, &duplicate], Compression::None)
            .unwrap();

    assert_eq!(
        source
            .layers()
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>(),
        ["roads", "roads"]
    );
}

#[cfg(feature = "gzip")]
#[test]
fn explicit_raw_compression_does_not_follow_gzip_magic() {
    // Given: a real gzip-encoded MVT sample.
    let encoded = common::gzip(&tile_with_layers(&["roads"]));

    // When: the caller explicitly declares the bytes raw.
    let result = MvtSource::from_mvt_with_compression("roads", &encoded, Compression::None);

    // Then: the explicit setting wins and the compressed bytes are invalid raw MVT.
    assert!(matches!(result, Err(SourceError::InvalidMvt)));
}

#[cfg(not(feature = "gzip"))]
#[test]
fn public_auto_constructor_reports_disabled_gzip() {
    assert!(matches!(
        MvtSource::from_mvt("gzip", &[0x1f, 0x8b, 0x08]),
        Err(SourceError::CompressionFeatureDisabled {
            compression: Compression::Gzip
        })
    ));
}

#[cfg(not(feature = "zstd"))]
#[test]
fn public_auto_constructor_reports_disabled_zstd() {
    assert!(matches!(
        MvtSource::from_mvt("zstd", &[0x28, 0xb5, 0x2f, 0xfd]),
        Err(SourceError::CompressionFeatureDisabled {
            compression: Compression::Zstd
        })
    ));
}

#[cfg(not(feature = "brotli"))]
#[test]
fn public_explicit_constructor_reports_disabled_brotli() {
    assert!(matches!(
        MvtSource::from_mvt_with_compression("brotli", b"encoded", Compression::Brotli,),
        Err(SourceError::CompressionFeatureDisabled {
            compression: Compression::Brotli
        })
    ));
}

#[test]
fn rejects_invalid_mvt_and_inconsistent_sample_compression() {
    assert!(matches!(
        MvtSource::from_mvt("invalid", b"not-an-mvt"),
        Err(SourceError::InvalidMvt)
    ));

    #[cfg(feature = "gzip")]
    {
        let raw = tile_with_layers(&["roads"]);
        let encoded = common::gzip(&raw);
        assert!(matches!(
            MvtSource::from_mvts("roads", [&raw, &encoded]),
            Err(SourceError::InconsistentSampleCompression { .. })
        ));
    }
}

#[cfg(feature = "gzip")]
#[test]
fn automatically_parses_gzip_sample() {
    let encoded = common::gzip(&tile_with_layers(&["roads"]));
    let source = MvtSource::from_mvt("roads", &encoded).unwrap();
    assert_eq!(source.compression(), Compression::Gzip);
    assert_eq!(source.layers()[0].as_ref(), "roads");
}

#[cfg(feature = "gzip")]
#[test]
fn parses_all_layers_from_concatenated_gzip_members() {
    // Given: two independent gzip members containing separate MVT layers.
    let first = common::gzip(&tile_with_layers(&["roads"]));
    let second = common::gzip(&tile_with_layers(&["buildings"]));
    let encoded: Vec<u8> = first.into_iter().chain(second).collect();

    // When: public source metadata parsing is requested.
    let source = MvtSource::from_mvt("transport", &encoded).unwrap();

    // Then: all concatenated members are decoded in order.
    assert_eq!(source.compression(), Compression::Gzip);
    assert_eq!(
        source
            .layers()
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>(),
        ["roads", "buildings"]
    );
}

#[cfg(feature = "zstd")]
#[test]
fn automatically_parses_zstd_sample() {
    let raw = tile_with_layers(&["roads"]);
    let encoded = zstd::stream::encode_all(&raw[..], 0).unwrap();
    let source = MvtSource::from_mvt("roads", &encoded).unwrap();
    assert_eq!(source.compression(), Compression::Zstd);
    assert_eq!(source.layers()[0].as_ref(), "roads");
}

#[cfg(feature = "brotli")]
#[test]
fn explicitly_parses_brotli_sample() {
    use std::io::Cursor;

    let raw = tile_with_layers(&["roads"]);
    let mut encoded = Vec::new();
    brotli::BrotliCompress(
        &mut Cursor::new(raw),
        &mut encoded,
        &brotli::enc::BrotliEncoderParams::default(),
    )
    .unwrap();
    let source =
        MvtSource::from_mvt_with_compression("roads", &encoded, Compression::Brotli).unwrap();
    assert_eq!(source.compression(), Compression::Brotli);
    assert_eq!(source.layers()[0].as_ref(), "roads");
}
