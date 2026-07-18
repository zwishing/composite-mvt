use crate::{Compression, MvtComposerBuilder, MvtSource};

pub struct MvtComposer {
    pub(crate) sources: Box<[MvtSource]>,
    pub(crate) output_compression: Compression,
}

impl MvtComposer {
    #[must_use]
    pub fn builder() -> MvtComposerBuilder {
        MvtComposerBuilder::default()
    }

    #[must_use]
    pub fn sources(&self) -> &[MvtSource] {
        &self.sources
    }

    #[must_use]
    pub const fn output_compression(&self) -> Compression {
        self.output_compression
    }
}
