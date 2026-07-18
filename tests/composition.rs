#[cfg(any(feature = "gzip", feature = "zstd", feature = "brotli"))]
mod common;
#[path = "composition/concurrency.rs"]
mod concurrency;
#[cfg(any(feature = "gzip", feature = "zstd", feature = "brotli"))]
#[path = "composition/e2e.rs"]
mod e2e;
#[cfg(any(feature = "gzip", feature = "zstd", feature = "brotli"))]
#[path = "composition/fixtures.rs"]
mod fixtures;

#[cfg(any(feature = "gzip", feature = "zstd", feature = "brotli"))]
use composite_mvt::Compression;
use composite_mvt::{ComposeError, MvtComposer, MvtSource};

fn composer(ids: &[&str]) -> MvtComposer {
    ids.iter()
        .fold(MvtComposer::builder(), |builder, id| {
            builder.add_source(MvtSource::new(*id).with_layers([*id]))
        })
        .build()
        .unwrap()
}

#[test]
fn composes_raw_inputs_in_source_order() {
    let composer = composer(&["a", "b", "c"]);

    let output = composer
        .compose(&[&b"first"[..], &b"second"[..], &b"third"[..]])
        .unwrap();

    assert_eq!(output.as_ref(), b"firstsecondthird");
}

#[test]
fn rejects_wrong_input_count_before_composition() {
    let composer = composer(&["a", "b"]);

    let missing = composer.compose(&[b"only"]);
    let extra = composer.compose(&[&b"one"[..], &b"two"[..], &b"three"[..]]);

    assert!(matches!(
        missing,
        Err(ComposeError::InputCountMismatch {
            expected: 2,
            actual: 1
        })
    ));
    assert!(matches!(
        extra,
        Err(ComposeError::InputCountMismatch {
            expected: 2,
            actual: 3
        })
    ));
}

#[test]
fn empty_inputs_are_preserved_without_mutating_sources() {
    let composer = composer(&["a", "b"]);
    let first = Vec::<u8>::new();
    let second = b"second".to_vec();

    let output = composer.compose(&[&first, &second]).unwrap();

    assert_eq!(output.as_ref(), b"second");
    assert!(first.is_empty());
    assert_eq!(second, b"second");
}

#[cfg(feature = "gzip")]
#[test]
fn decompresses_each_source_before_composing() {
    let first = common::tile_with_layers(&["roads"]);
    let second = common::tile_with_layers(&["water"]);
    let encoded_first = common::gzip(&first);
    let composer = MvtComposer::builder()
        .add_source(
            MvtSource::new("roads")
                .with_compression(Compression::Gzip)
                .with_layers(["roads"]),
        )
        .add_source(MvtSource::new("water").with_layers(["water"]))
        .build()
        .unwrap();

    let output = composer
        .compose(&[encoded_first.as_slice(), second.as_slice()])
        .unwrap();
    let expected: Vec<u8> = first.into_iter().chain(second).collect();

    assert_eq!(output.as_ref(), expected);
}

#[cfg(feature = "gzip")]
#[test]
fn reports_the_source_that_failed_decompression() {
    let composer = MvtComposer::builder()
        .add_source(
            MvtSource::new("roads")
                .with_compression(composite_mvt::Compression::Gzip)
                .with_layers(["roads"]),
        )
        .build()
        .unwrap();

    let error = composer.compose(&[b"not-gzip"]).unwrap_err();

    assert!(matches!(
        error,
        ComposeError::SourceDecompression { source_id, .. }
            if source_id.as_ref() == "roads"
    ));
}

#[cfg(feature = "gzip")]
#[test]
fn compresses_the_complete_output_with_dependency_defaults() {
    let first = common::tile_with_layers(&["roads"]);
    let second = common::tile_with_layers(&["water"]);
    let expected_raw: Vec<u8> = first.iter().chain(&second).copied().collect();
    let expected = common::gzip(&expected_raw);
    let composer = MvtComposer::builder()
        .add_source(MvtSource::new("roads").with_layers(["roads"]))
        .add_source(MvtSource::new("water").with_layers(["water"]))
        .output_compression(Compression::Gzip)
        .build()
        .unwrap();

    let output = composer.compose(&[first, second]).unwrap();

    assert_eq!(output.as_ref(), expected);
}

#[cfg(feature = "zstd")]
#[test]
fn compresses_the_complete_output_with_zstd_defaults() {
    let first = common::tile_with_layers(&["roads"]);
    let second = common::tile_with_layers(&["water"]);
    let expected_raw: Vec<u8> = first.iter().chain(&second).copied().collect();
    let expected = fixtures::zstd_encode(&expected_raw);
    let composer = MvtComposer::builder()
        .add_source(MvtSource::new("roads").with_layers(["roads"]))
        .add_source(MvtSource::new("water").with_layers(["water"]))
        .output_compression(Compression::Zstd)
        .build()
        .unwrap();

    let output = composer.compose(&[first, second]).unwrap();

    assert_eq!(output.as_ref(), expected);
}

#[cfg(feature = "brotli")]
#[test]
fn compresses_the_complete_output_with_brotli_defaults() {
    let first = common::tile_with_layers(&["roads"]);
    let second = common::tile_with_layers(&["water"]);
    let expected_raw: Vec<u8> = first.iter().chain(&second).copied().collect();
    let expected = fixtures::brotli_encode(&expected_raw);
    let composer = MvtComposer::builder()
        .add_source(MvtSource::new("roads").with_layers(["roads"]))
        .add_source(MvtSource::new("water").with_layers(["water"]))
        .output_compression(Compression::Brotli)
        .build()
        .unwrap();

    let output = composer.compose(&[first, second]).unwrap();

    assert_eq!(output.as_ref(), expected);
}
