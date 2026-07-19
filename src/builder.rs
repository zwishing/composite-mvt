use std::collections::{HashMap, HashSet};

use crate::compression::feature_enabled;
use crate::{BuildError, Compression, DuplicateLayer, LayerName, MvtComposer, MvtSource, SourceId};

/// Configures the immutable source list and output encoding for an [`MvtComposer`].
///
/// Source configuration is fixed when [`Self::build`] succeeds. Request bytes are supplied later
/// to [`MvtComposer::compose`], in the same order as sources are added here.
pub struct MvtComposerBuilder {
    sources: Vec<MvtSource>,
    duplicate_layer: DuplicateLayer,
    output_compression: Compression,
}

impl Default for MvtComposerBuilder {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            duplicate_layer: DuplicateLayer::Error,
            output_compression: Compression::None,
        }
    }
}

impl MvtComposerBuilder {
    /// Sets how equal layer names in separate sources are handled.
    ///
    /// [`DuplicateLayer::Error`] is the default. A duplicate within one source is always an error,
    /// regardless of this policy.
    #[must_use]
    pub fn duplicate_layer(mut self, behavior: DuplicateLayer) -> Self {
        self.duplicate_layer = behavior;
        self
    }

    /// Sets the encoding applied once to the complete composite output.
    ///
    /// This setting is fixed in the built composer; it is not selected per request. The required
    /// Cargo feature must be enabled for a non-raw encoding.
    #[must_use]
    pub fn output_compression(mut self, compression: Compression) -> Self {
        self.output_compression = compression;
        self
    }

    /// Adds a source to the fixed input order.
    ///
    /// Each [`MvtComposer::compose`] call must provide exactly one byte slice for every source, in
    /// this order.
    #[must_use]
    pub fn add_source(mut self, source: MvtSource) -> Self {
        self.sources.push(source);
        self
    }

    /// Validates duplicate layer names without consuming or changing this builder.
    ///
    /// A duplicate within one source always returns [`BuildError::DuplicateLayerName`]. A duplicate
    /// across sources returns that error only under [`DuplicateLayer::Error`]; it is accepted under
    /// [`DuplicateLayer::Allow`]. [`Self::build`] reuses this validation.
    pub fn validate_duplicate_layers(&self) -> Result<(), BuildError> {
        let mut global: HashMap<&LayerName, &SourceId> = HashMap::new();

        for source in &self.sources {
            let mut local = HashSet::new();
            for layer in source.layers() {
                if !local.insert(layer) {
                    return Err(BuildError::DuplicateLayerName {
                        layer: layer.clone(),
                        first_source: source.id().clone(),
                        second_source: source.id().clone(),
                    });
                }

                if let Some(first_source) = global.get(layer) {
                    if self.duplicate_layer == DuplicateLayer::Error {
                        return Err(BuildError::DuplicateLayerName {
                            layer: layer.clone(),
                            first_source: (*first_source).clone(),
                            second_source: source.id().clone(),
                        });
                    }
                } else {
                    global.insert(layer, source.id());
                }
            }
        }

        Ok(())
    }

    /// Validates all configuration and constructs an immutable, lock-free composer.
    ///
    /// In addition to duplicate-layer validation, this checks source IDs, required source layers,
    /// and feature support for source and output encodings.
    pub fn build(self) -> Result<MvtComposer, BuildError> {
        self.validate()?;

        Ok(MvtComposer {
            sources: self.sources.into_boxed_slice(),
            output_compression: self.output_compression,
        })
    }

    fn validate(&self) -> Result<(), BuildError> {
        if self.sources.is_empty() {
            return Err(BuildError::NoSources);
        }

        let mut ids = HashSet::new();
        for source in &self.sources {
            if !ids.insert(source.id()) {
                return Err(BuildError::DuplicateSourceId {
                    id: source.id().clone(),
                });
            }
        }

        for source in &self.sources {
            if source.layers().is_empty() {
                return Err(BuildError::NoLayers {
                    source_id: source.id().clone(),
                });
            }
        }

        for source in &self.sources {
            if source
                .layers()
                .iter()
                .any(|layer| layer.as_ref().is_empty())
            {
                return Err(BuildError::EmptyLayerName {
                    source_id: source.id().clone(),
                });
            }
        }

        self.validate_duplicate_layers()?;

        for source in &self.sources {
            validate_source_compression(source)?;
        }

        validate_output_compression(self.output_compression)
    }
}

fn validate_source_compression(source: &MvtSource) -> Result<(), BuildError> {
    let compression = source.compression();
    if compression == Compression::Other {
        return Err(BuildError::UnsupportedCompression {
            source_id: source.id().clone(),
            compression,
        });
    }
    if !feature_enabled(compression) {
        return Err(BuildError::CompressionFeatureDisabled {
            source_id: source.id().clone(),
            compression,
        });
    }

    Ok(())
}

fn validate_output_compression(compression: Compression) -> Result<(), BuildError> {
    if compression == Compression::Other {
        return Err(BuildError::UnsupportedOutputCompression { compression });
    }
    if !feature_enabled(compression) {
        return Err(BuildError::OutputCompressionFeatureDisabled { compression });
    }

    Ok(())
}
