use std::collections::{HashMap, HashSet};

use crate::compression::feature_enabled;
use crate::{BuildError, Compression, DuplicateLayer, LayerName, MvtComposer, MvtSource, SourceId};

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
    #[must_use]
    pub fn duplicate_layer(mut self, behavior: DuplicateLayer) -> Self {
        self.duplicate_layer = behavior;
        self
    }

    #[must_use]
    pub fn output_compression(mut self, compression: Compression) -> Self {
        self.output_compression = compression;
        self
    }

    #[must_use]
    pub fn add_source(mut self, source: MvtSource) -> Self {
        self.sources.push(source);
        self
    }

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
            if source.layers().is_empty() {
                return Err(BuildError::NoLayers {
                    source_id: source.id().clone(),
                });
            }
            if source
                .layers()
                .iter()
                .any(|layer| layer.as_ref().is_empty())
            {
                return Err(BuildError::EmptyLayerName {
                    source_id: source.id().clone(),
                });
            }
            validate_source_compression(source)?;
        }

        self.validate_duplicate_layers()?;
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
