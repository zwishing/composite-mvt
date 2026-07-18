mod common;

use common::tile_with_layers;
use composite_mvt::{Compression, MvtSource, SourceError};

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
