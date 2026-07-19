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

    let Err(BuildError::DuplicateSourceId { id }) = result else {
        panic!("expected duplicate source ID");
    };
    assert_eq!(id.as_ref(), "roads");
}

#[test]
fn explicit_duplicate_validation_matches_build() {
    let builder = MvtComposer::builder()
        .duplicate_layer(DuplicateLayer::Error)
        .add_source(source("a", &["shared"]))
        .add_source(source("b", &["shared"]));

    let Err(BuildError::DuplicateLayerName {
        layer,
        first_source,
        second_source,
    }) = builder.validate_duplicate_layers()
    else {
        panic!("expected a cross-source duplicate layer");
    };

    assert_eq!(layer.as_ref(), "shared");
    assert_eq!(first_source.as_ref(), "a");
    assert_eq!(second_source.as_ref(), "b");
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

    let Err(BuildError::DuplicateLayerName {
        layer,
        first_source,
        second_source,
    }) = builder.validate_duplicate_layers()
    else {
        panic!("expected a same-source duplicate layer");
    };

    assert_eq!(layer.as_ref(), "roads");
    assert_eq!(first_source.as_ref(), "roads");
    assert_eq!(second_source.as_ref(), "roads");
}

#[test]
fn duplicate_source_ids_precede_per_source_errors() {
    // Given: the first source is incomplete and a later source repeats its ID.
    let builder = MvtComposer::builder()
        .add_source(source("same", &[""]))
        .add_source(MvtSource::new("same"));

    // When: the complete builder is validated.
    let result = builder.build();

    // Then: the global source-ID pass wins over source-local errors.
    assert!(matches!(
        result,
        Err(BuildError::DuplicateSourceId { id }) if id.as_ref() == "same"
    ));
}

#[test]
fn missing_layers_across_all_sources_precede_earlier_empty_names() {
    // Given: an early source has an empty name and a later source has no layers.
    let builder = MvtComposer::builder()
        .add_source(source("early", &[""]))
        .add_source(MvtSource::new("later"));

    // When: the complete builder is validated.
    let result = builder.build();

    // Then: the all-sources no-layer pass completes before empty names are checked.
    assert!(matches!(
        result,
        Err(BuildError::NoLayers { source_id }) if source_id.as_ref() == "later"
    ));
}

#[test]
fn duplicate_layers_precede_unsupported_source_compression() {
    // Given: the second source duplicates a layer and uses an unsupported codec.
    let builder = MvtComposer::builder()
        .add_source(source("first", &["shared"]))
        .add_source(source("second", &["shared"]).with_compression(Compression::Other));

    // When: the complete builder is validated.
    let result = builder.build();

    // Then: duplicate validation runs before source compression validation.
    let Err(BuildError::DuplicateLayerName {
        layer,
        first_source,
        second_source,
    }) = result
    else {
        panic!("expected duplicate layer precedence");
    };
    assert_eq!(layer.as_ref(), "shared");
    assert_eq!(first_source.as_ref(), "first");
    assert_eq!(second_source.as_ref(), "second");
}

#[cfg(not(feature = "zstd"))]
#[test]
fn duplicate_layers_precede_disabled_source_compression() {
    // Given: the second source duplicates a layer and requires disabled zstd support.
    let builder = MvtComposer::builder()
        .add_source(source("first", &["shared"]))
        .add_source(source("second", &["shared"]).with_compression(Compression::Zstd));

    // When: the complete builder is validated.
    let result = builder.build();

    // Then: duplicate validation runs before feature validation.
    assert!(matches!(
        result,
        Err(BuildError::DuplicateLayerName {
            layer,
            first_source,
            second_source,
        }) if layer.as_ref() == "shared"
            && first_source.as_ref() == "first"
            && second_source.as_ref() == "second"
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
