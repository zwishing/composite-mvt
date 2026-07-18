use std::borrow::Cow;

use bytes::{Bytes, BytesMut};

use crate::compression::compress;
use crate::{ComposeError, Compression, MvtComposerBuilder, MvtSource};

fn checked_total_len<'a>(
    inputs: impl IntoIterator<Item = &'a [u8]>,
) -> Result<usize, ComposeError> {
    checked_total_from_lengths(inputs.into_iter().map(<[u8]>::len))
}

fn checked_total_from_lengths(
    lengths: impl IntoIterator<Item = usize>,
) -> Result<usize, ComposeError> {
    lengths.into_iter().try_fold(0usize, |total, length| {
        total.checked_add(length).ok_or(ComposeError::SizeOverflow)
    })
}

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

    pub fn compose<B>(&self, inputs: &[B]) -> Result<Bytes, ComposeError>
    where
        B: AsRef<[u8]>,
    {
        if inputs.len() != self.sources.len() {
            return Err(ComposeError::InputCountMismatch {
                expected: self.sources.len(),
                actual: inputs.len(),
            });
        }

        let raw_inputs: Vec<Cow<'_, [u8]>> = self
            .sources
            .iter()
            .zip(inputs)
            .map(|(source, input)| {
                source.decompress(input.as_ref()).map_err(|source_error| {
                    ComposeError::SourceDecompression {
                        source_id: source.id().clone(),
                        source: source_error,
                    }
                })
            })
            .collect::<Result<_, _>>()?;
        let raw = self.compose_raw(&raw_inputs)?;

        match self.output_compression {
            Compression::None => Ok(raw),
            compression @ (Compression::Gzip
            | Compression::Zstd
            | Compression::Brotli
            | Compression::Other) => {
                compress(compression, &raw).map_err(|source| ComposeError::OutputCompression {
                    compression,
                    source,
                })
            }
        }
    }

    fn compose_raw<B>(&self, raw_inputs: &[B]) -> Result<Bytes, ComposeError>
    where
        B: AsRef<[u8]>,
    {
        let total = checked_total_len(raw_inputs.iter().map(AsRef::as_ref))?;
        let mut output = BytesMut::with_capacity(total);
        for input in raw_inputs {
            output.extend_from_slice(input.as_ref());
        }
        Ok(output.freeze())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_total_rejects_overflow() {
        assert!(matches!(
            checked_total_from_lengths([usize::MAX, 1]),
            Err(ComposeError::SizeOverflow)
        ));
    }
}
