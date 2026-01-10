use crate::shadow::ShadowError;

/// Buffer for staging writes before committing to the shadow table.
pub trait StagingBuffer {
    /// Returns true if any writes are staged.
    fn any_staged(&self) -> bool;

    /// Zero-copy staged write access.
    ///
    /// Pre-allocates space and provides a mutable slice.
    /// Return `true` to commit the staged write, `false` to reclaim space.
    fn alloc_staged(
        &mut self,
        addr: u16,
        len: usize,
        f: impl FnOnce(&mut [u8]) -> bool,
    ) -> Result<bool, ShadowError>;

    /// Clears all staged writes.
    fn clear_staged(&mut self) -> Result<(), ShadowError>;

    /// Iterates over all staged writes in order.
    fn iter_staged<F>(&self, f: F) -> Result<(), ShadowError>
    where
        F: FnMut(u16, &[u8]) -> Result<(), ShadowError>;
}
