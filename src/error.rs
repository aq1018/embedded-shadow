#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowError {
    OutOfBounds,
    ZeroLength,
    Denied,
    StageFull,
}
