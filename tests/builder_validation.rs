use composite_mvt::{BuildError, Compression, DuplicateLayer, MvtComposer, MvtSource};

fn source(id: &str, layers: &[&str]) -> MvtSource {
    MvtSource::new(id).with_layers(layers.iter().copied())
}

#[test]
fn rejects_no_sources_and_duplicate_ids() {
    assert!(matches!(
        MvtComposer::builder().build(),
        Err(BuildError::NoSources)
    ));

    let result = MvtComposer::builder()
        .add_source(source("roads", &["roads"]))
        .add_source(source("roads", &["labels"]))
        .build();

    assert!(matches!(result, Err(BuildError::DuplicateSourceId { .. })));
}

#[test]
fn explicit_duplicate_validation_matches_build() {
    let builder = MvtComposer::builder()
        .duplicate_layer(DuplicateLayer::Error)
        .add_source(source("a", &["shared"]))
        .add_source(source("b", &["shared"]));

    assert!(matches!(
        builder.validate_duplicate_layers(),
        Err(BuildError::DuplicateLayerName { .. })
    ));
    assert!(matches!(
        builder.build(),
        Err(BuildError::DuplicateLayerName { .. })
    ));
}

#[test]
fn allows_cross_source_duplicates_when_configured() {
    let composer = MvtComposer::builder()
        .duplicate_layer(DuplicateLayer::Allow)
        .add_source(source("a", &["shared"]))
        .add_source(source("b", &["shared"]))
        .build()
        .unwrap();

    assert_eq!(composer.sources().len(), 2);
    assert_eq!(composer.output_compression(), Compression::None);
}

#[test]
fn rejects_missing_and_empty_layers() {
    assert!(matches!(
        MvtComposer::builder()
            .add_source(MvtSource::new("empty"))
            .build(),
        Err(BuildError::NoLayers { .. })
    ));
    assert!(matches!(
        MvtComposer::builder()
            .add_source(source("empty", &[""]))
            .build(),
        Err(BuildError::EmptyLayerName { .. })
    ));
}

#[test]
fn same_source_duplicates_are_always_invalid() {
    let builder = MvtComposer::builder()
        .duplicate_layer(DuplicateLayer::Allow)
        .add_source(source("roads", &["roads", "roads"]));

    assert!(matches!(
        builder.validate_duplicate_layers(),
        Err(BuildError::DuplicateLayerName { .. })
    ));
}

#[test]
fn rejects_other_for_input_and_output() {
    assert!(matches!(
        MvtComposer::builder()
            .add_source(source("roads", &["roads"]).with_compression(Compression::Other))
            .build(),
        Err(BuildError::UnsupportedCompression { .. })
    ));
    assert!(matches!(
        MvtComposer::builder()
            .output_compression(Compression::Other)
            .add_source(source("roads", &["roads"]))
            .build(),
        Err(BuildError::UnsupportedOutputCompression { .. })
    ));
}

#[cfg(not(feature = "zstd"))]
#[test]
fn rejects_disabled_input_and_output_features() {
    assert!(matches!(
        MvtComposer::builder()
            .add_source(source("roads", &["roads"]).with_compression(Compression::Zstd))
            .build(),
        Err(BuildError::CompressionFeatureDisabled { .. })
    ));
    assert!(matches!(
        MvtComposer::builder()
            .output_compression(Compression::Zstd)
            .add_source(source("roads", &["roads"]))
            .build(),
        Err(BuildError::OutputCompressionFeatureDisabled { .. })
    ));
}

#[test]
fn preserves_source_order() {
    let composer = MvtComposer::builder()
        .add_source(source("roads", &["roads"]))
        .add_source(source("buildings", &["building"]))
        .build()
        .unwrap();

    assert_eq!(
        composer
            .sources()
            .iter()
            .map(|source| source.id().as_ref())
            .collect::<Vec<_>>(),
        ["roads", "buildings"]
    );
}
