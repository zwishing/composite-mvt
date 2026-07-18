/// Policy for repeated layer names across distinct sources.
///
/// The builder always rejects a duplicate within one source. This policy only controls a layer
/// name that occurs in two or more different sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DuplicateLayer {
    /// Allow distinct sources to declare the same layer name.
    Allow,
    /// Reject a layer name shared by distinct sources. This is the default policy.
    #[default]
    Error,
}
