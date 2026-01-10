use crate::shadow::{
    ShadowError,
    slice::{ROSlice, RWSlice},
    table::ShadowTable,
};

/// Hardware/kernel-side view of the shadow table.
///
/// Provides read/write access without marking blocks dirty, plus
/// methods to query and clear dirty state. Used by hardware drivers
/// to sync shadow data to/from actual hardware registers.
pub struct KernelView<'a, const TS: usize, const BS: usize, const BC: usize>
where
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    table: &'a mut ShadowTable<TS, BS, BC>,
}

impl<'a, const TS: usize, const BS: usize, const BC: usize> core::fmt::Debug
    for KernelView<'a, TS, BS, BC>
where
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("KernelView").finish_non_exhaustive()
    }
}

impl<'a, const TS: usize, const BS: usize, const BC: usize> KernelView<'a, TS, BS, BC>
where
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    pub(crate) fn new(table: &'a mut ShadowTable<TS, BS, BC>) -> Self {
        Self { table }
    }
}

impl<'a, const TS: usize, const BS: usize, const BC: usize> KernelView<'a, TS, BS, BC>
where
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    /// Provides zero-copy read access via ROSlice without marking clean.
    pub fn with_ro_slice<F, R>(&self, addr: u16, len: usize, f: F) -> Result<R, ShadowError>
    where
        F: FnOnce(ROSlice<'_>) -> R,
    {
        self.table
            .with_bytes(addr, len, |data| Ok(f(ROSlice::new(data))))
    }

    /// Provides zero-copy read-write access via RWSlice without marking dirty.
    pub fn with_rw_slice<F, R>(&mut self, addr: u16, len: usize, f: F) -> Result<R, ShadowError>
    where
        F: FnOnce(RWSlice<'_>) -> R,
    {
        self.table
            .with_bytes_mut(addr, len, |data| Ok(f(RWSlice::new(data))))
    }

    /// Iterates over each dirty block, providing its address and data as ROSlice.
    pub fn iter_dirty<F>(&self, mut f: F) -> Result<(), ShadowError>
    where
        F: FnMut(u16, ROSlice<'_>) -> Result<(), ShadowError>,
    {
        self.table
            .iter_dirty(|addr, data| f(addr, ROSlice::new(data)))
    }

    /// Marks all blocks overlapping the given range as clean.
    pub fn mark_clean(&mut self, addr: u16, len: usize) -> Result<(), ShadowError> {
        self.table.mark_clean(addr, len)
    }

    /// Returns true if any block overlapping the given range is dirty.
    pub fn is_dirty(&self, addr: u16, len: usize) -> Result<bool, ShadowError> {
        self.table.is_dirty(addr, len)
    }

    /// Returns true if any block in the table is dirty.
    pub fn any_dirty(&self) -> bool {
        self.table.any_dirty()
    }

    /// Clears all dirty flags in the table.
    pub fn clear_dirty(&mut self) {
        self.table.clear_dirty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shadow::test_support::TestTable;

    #[test]
    fn kernel_read_does_not_mark_dirty() {
        let mut table = TestTable::new();
        let view = KernelView::new(&mut table);

        view.with_ro_slice(0, 4, |_slice| {}).unwrap();

        assert!(!view.any_dirty());
    }

    #[test]
    fn kernel_write_does_not_mark_dirty() {
        let mut table = TestTable::new();
        let mut view = KernelView::new(&mut table);

        view.with_rw_slice(0, 4, |mut slice| {
            slice.copy_from_slice(&[0xFF; 4]);
        })
        .unwrap();

        assert!(!view.any_dirty());

        // Verify data was actually written
        view.with_ro_slice(0, 4, |slice| {
            let mut buf = [0u8; 4];
            slice.copy_to_slice(&mut buf);
            assert_eq!(buf, [0xFF; 4]);
        })
        .unwrap();
    }

    #[test]
    fn kernel_clear_dirty_clears_all_blocks() {
        let mut table = TestTable::new();
        // Manually mark some blocks dirty
        table.mark_dirty(0, 16).unwrap();
        table.mark_dirty(32, 16).unwrap();

        let mut view = KernelView::new(&mut table);
        assert!(view.any_dirty());

        view.clear_dirty();

        assert!(!view.any_dirty());
    }

    #[test]
    fn iter_dirty_iterates_only_dirty() {
        let mut table = TestTable::new();
        // Mark only block 0 (bytes 0-15) and block 2 (bytes 32-47) dirty
        table.mark_dirty(0, 16).unwrap();
        table.mark_dirty(32, 16).unwrap();

        let view = KernelView::new(&mut table);

        let mut count = 0;
        let mut addrs = [0u16; 4];
        view.iter_dirty(|addr, _data| {
            addrs[count] = addr;
            count += 1;
            Ok(())
        })
        .unwrap();

        assert_eq!(count, 2);
        assert_eq!(addrs[0], 0);
        assert_eq!(addrs[1], 32);
    }

    #[test]
    fn iter_dirty_provides_correct_data() {
        let mut table = TestTable::new();
        table
            .with_bytes_mut(0, 16, |buf| {
                buf.copy_from_slice(&[0xAA; 16]);
                Ok(())
            })
            .unwrap();
        table.mark_dirty(0, 16).unwrap();

        let view = KernelView::new(&mut table);

        view.iter_dirty(|addr, slice| {
            assert_eq!(addr, 0);
            assert_eq!(slice.len(), 16);
            // Check data using ROSlice primitives
            for i in 0..16 {
                assert_eq!(slice.read_u8_at(i), 0xAA);
            }
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn is_dirty_partial_overlap_returns_true() {
        let mut table = TestTable::new();
        // Mark block 0 (bytes 0-15) dirty
        table.mark_dirty(0, 16).unwrap();

        let view = KernelView::new(&mut table);

        // Query overlaps with dirty block
        assert!(view.is_dirty(8, 8).unwrap());
        // Query is entirely within dirty block
        assert!(view.is_dirty(0, 4).unwrap());
        // Query spans dirty and clean blocks
        assert!(view.is_dirty(8, 16).unwrap());
        // Query is entirely in clean block
        assert!(!view.is_dirty(16, 8).unwrap());
    }

    #[test]
    fn mark_clean_partial_block_clears_whole_block() {
        let mut table = TestTable::new();
        // Mark block 0 (bytes 0-15) dirty
        table.mark_dirty(0, 16).unwrap();

        let mut view = KernelView::new(&mut table);

        // Mark clean with partial range (addr=5, len=1) should clear entire block 0
        view.mark_clean(5, 1).unwrap();

        // Entire block 0 should be clean now
        assert!(!view.is_dirty(0, 16).unwrap());
    }

    #[test]
    fn iter_dirty_stops_on_first_error() {
        let mut table = TestTable::new();
        // Mark 3 blocks dirty
        table.mark_dirty(0, 16).unwrap();
        table.mark_dirty(16, 16).unwrap();
        table.mark_dirty(32, 16).unwrap();

        let view = KernelView::new(&mut table);

        let mut count = 0;
        let result = view.iter_dirty(|_addr, _data| {
            count += 1;
            if count == 2 {
                Err(ShadowError::OutOfBounds) // Simulate error on second block
            } else {
                Ok(())
            }
        });

        // Should have stopped on error and propagated it
        assert!(result.is_err());
        assert_eq!(count, 2); // Only processed 2 blocks before error
    }
}
