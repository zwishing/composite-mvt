#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DuplicateLayer {
    Allow,
    #[default]
    Error,
}
