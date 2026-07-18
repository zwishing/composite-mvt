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

/// An immutable composition plan for fixed MVT sources.
///
/// A composer contains only owned configuration and can be shared as `Arc<MvtComposer>` without
/// library-managed locks. Each [`Self::compose`] call owns its decompression and output buffers.
pub struct MvtComposer {
    pub(crate) sources: Box<[MvtSource]>,
    pub(crate) output_compression: Compression,
}

impl MvtComposer {
    /// Starts configuring a composer.
    #[must_use]
    pub fn builder() -> MvtComposerBuilder {
        MvtComposerBuilder::default()
    }

    /// Returns the fixed sources in the input order used by [`Self::compose`].
    #[must_use]
    pub fn sources(&self) -> &[MvtSource] {
        &self.sources
    }

    /// Returns the fixed output encoding selected during building.
    #[must_use]
    pub const fn output_compression(&self) -> Compression {
        self.output_compression
    }

    /// Composes one input per configured source, preserving source order.
    ///
    /// `inputs[n]` is decoded according to `self.sources()[n].compression()`. Raw source inputs
    /// are borrowed while compressed inputs are decoded before the internal raw-only merge. The
    /// merged bytes are then returned raw or encoded once using [`Self::output_compression`].
    ///
    /// A gzip output is one gzip member containing the complete composite MVT, suitable for an
    /// HTTP response with `Content-Encoding: gzip`. This crate returns bytes only; callers set HTTP
    /// headers themselves via [`Compression::content_encoding`].
    ///
    /// # Errors
    ///
    /// Returns [`ComposeError::InputCountMismatch`] when the input count differs from the fixed
    /// source count, [`ComposeError::SourceDecompression`] when a configured input cannot be
    /// decoded, or a compression/size error while producing the result.
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
