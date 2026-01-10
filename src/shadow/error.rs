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

impl core::fmt::Display for ShadowError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ShadowError::OutOfBounds => write!(f, "address or length exceeds table bounds"),
            ShadowError::ZeroLength => write!(f, "operation attempted with zero length"),
            ShadowError::Denied => write!(f, "access denied by policy"),
            ShadowError::StageFull => write!(f, "staging buffer capacity exceeded"),
        }
    }
}
