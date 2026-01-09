use crate::table::ShadowTable;

pub struct KernelView<'a, const TS: usize, const BS: usize, const BC: usize>
where
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    table: &'a mut ShadowTable<TS, BS, BC>,
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
    pub fn read_range(&self, addr: u16, out: &mut [u8]) -> Result<(), crate::ShadowError> {
        self.table.read_range(addr, out)
    }

    pub fn write_range(&mut self, addr: u16, data: &[u8]) -> Result<(), crate::ShadowError> {
        self.table.write_range(addr, data)?;
        Ok(())
    }

    pub fn for_each_dirty_block<F>(&self, mut f: F) -> Result<(), crate::ShadowError>
    where
        F: FnMut(u16, &[u8]) -> Result<(), crate::ShadowError>,
    {
        self.table.for_each_dirty_block(|addr, data| f(addr, data))
    }

    pub fn is_dirty(&self, addr: u16, len: usize) -> Result<bool, crate::ShadowError> {
        self.table.is_dirty(addr, len)
    }

    pub fn any_dirty(&self) -> bool {
        self.table.any_dirty()
    }

    pub fn clear_dirty(&mut self) {
        self.table.clear_dirty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestTable;

    #[test]
    fn kernel_read_does_not_mark_dirty() {
        let mut table = TestTable::new();
        let view = KernelView::new(&mut table);

        let mut buf = [0u8; 4];
        view.read_range(0, &mut buf).unwrap();

        assert!(!view.any_dirty());
    }

    #[test]
    fn kernel_write_does_not_mark_dirty() {
        let mut table = TestTable::new();
        let mut view = KernelView::new(&mut table);

        view.write_range(0, &[0xFF; 4]).unwrap();

        assert!(!view.any_dirty());

        // Verify data was actually written
        let mut buf = [0u8; 4];
        view.read_range(0, &mut buf).unwrap();
        assert_eq!(buf, [0xFF; 4]);
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
    fn for_each_dirty_block_iterates_only_dirty() {
        let mut table = TestTable::new();
        // Mark only block 0 (bytes 0-15) and block 2 (bytes 32-47) dirty
        table.mark_dirty(0, 16).unwrap();
        table.mark_dirty(32, 16).unwrap();

        let view = KernelView::new(&mut table);

        let mut count = 0;
        let mut addrs = [0u16; 4];
        view.for_each_dirty_block(|addr, _data| {
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
    fn for_each_dirty_block_provides_correct_data() {
        let mut table = TestTable::new();
        table.write_range(0, &[0xAA; 16]).unwrap();
        table.mark_dirty(0, 16).unwrap();

        let view = KernelView::new(&mut table);

        view.for_each_dirty_block(|addr, data| {
            assert_eq!(addr, 0);
            assert_eq!(data.len(), 16);
            assert!(data.iter().all(|&b| b == 0xAA));
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
    fn any_dirty_returns_false_after_clear() {
        let mut table = TestTable::new();
        table.mark_dirty(0, 64).unwrap(); // Mark all blocks

        let mut view = KernelView::new(&mut table);
        assert!(view.any_dirty());

        view.clear_dirty();
        assert!(!view.any_dirty());
    }
}
