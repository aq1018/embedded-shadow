use crate::shadow::ShadowError;

/// Buffer for staging writes before committing to the shadow table.
pub trait StagingBuffer {
    /// Returns true if any writes are staged.
    fn any_staged(&self) -> bool;
    /// Applies staged writes to the output buffer for the given address range.
    fn apply_overlay(&self, addr: u16, out: &mut [u8]) -> Result<(), ShadowError>;
    /// Stages a write to be applied on commit.
    fn write_staged(&mut self, addr: u16, data: &[u8]) -> Result<(), ShadowError>;
    /// Clears all staged writes.
    fn clear_staged(&mut self) -> Result<(), ShadowError>;
    /// Iterates over all staged writes in order.
    fn for_each_staged<F>(&self, f: F) -> Result<(), ShadowError>
    where
        F: FnMut(u16, &[u8]) -> Result<(), ShadowError>;
}
