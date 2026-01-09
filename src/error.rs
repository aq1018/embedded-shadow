/// Errors that can occur during shadow table operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowError {
    /// Address or length exceeds table bounds.
    OutOfBounds,
    /// Operation attempted with zero length.
    ZeroLength,
    /// Access denied by policy.
    Denied,
    /// Staging buffer capacity exceeded.
    StageFull,
}
