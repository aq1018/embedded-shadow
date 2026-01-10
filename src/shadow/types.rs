use crate::shadow::ShadowError;

/// Result of a write operation indicating whether to mark blocks dirty.
///
/// Used as the return type for write callbacks to clearly indicate intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteResult<R> {
    /// Mark the written range as dirty and return the result.
    Dirty(R),
    /// Do not mark dirty; return the result.
    Clean(R),
}

impl<R> WriteResult<R> {
    /// Returns true if this result indicates dirty.
    #[inline]
    pub fn is_dirty(&self) -> bool {
        matches!(self, WriteResult::Dirty(_))
    }

    /// Unwraps the inner value regardless of dirty state.
    #[inline]
    pub fn into_inner(self) -> R {
        match self {
            WriteResult::Dirty(r) | WriteResult::Clean(r) => r,
        }
    }
}

/// Buffer for staging writes before committing to the shadow table.
pub trait StagingBuffer {
    /// Returns true if any writes are staged.
    fn any_staged(&self) -> bool;

    /// Zero-copy staged write access.
    ///
    /// Pre-allocates space and provides a mutable slice.
    /// Return `WriteResult::Dirty(())` to commit the staged write,
    /// `WriteResult::Clean(())` to reclaim space without committing.
    fn alloc_staged(
        &mut self,
        addr: u16,
        len: usize,
        f: impl FnOnce(&mut [u8]) -> WriteResult<()>,
    ) -> Result<WriteResult<()>, ShadowError>;

    /// Clears all staged writes.
    fn clear_staged(&mut self) -> Result<(), ShadowError>;

    /// Iterates over all staged writes in order.
    fn iter_staged<F>(&self, f: F) -> Result<(), ShadowError>
    where
        F: FnMut(u16, &[u8]) -> Result<(), ShadowError>;
}
