use crate::shadow::{
    error::ShadowError,
    helpers::{block_span, range_span},
};

pub(crate) struct ShadowTable<const TS: usize, const BS: usize, const BC: usize>
where
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    bytes: [u8; TS],
    dirty: bitmaps::Bitmap<BC>,
}

impl<const TS: usize, const BS: usize, const BC: usize> ShadowTable<TS, BS, BC>
where
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    pub(crate) fn new() -> Self {
        debug_assert!(
            TS == BS * BC,
            "Total size must match block size x block count",
        );

        Self {
            bytes: [0; TS],
            dirty: bitmaps::Bitmap::new(),
        }
    }

    pub(crate) fn with_bytes<F, R>(&self, addr: u16, len: usize, f: F) -> Result<R, ShadowError>
    where
        F: FnOnce(&[u8]) -> Result<R, ShadowError>,
    {
        let (offset, end) = range_span::<TS>(addr, len)?;
        f(&self.bytes[offset..end])
    }

    pub(crate) fn with_bytes_mut<F, R>(
        &mut self,
        addr: u16,
        len: usize,
        f: F,
    ) -> Result<R, ShadowError>
    where
        F: FnOnce(&mut [u8]) -> Result<R, ShadowError>,
    {
        let (offset, end) = range_span::<TS>(addr, len)?;
        f(&mut self.bytes[offset..end])
    }

    pub(crate) fn iter_dirty<F>(&self, mut f: F) -> Result<(), ShadowError>
    where
        F: FnMut(u16, &[u8]) -> Result<(), ShadowError>,
    {
        let mut idx = self.dirty.first_index();
        while let Some(block) = idx {
            let off = block * BS;
            let buf = &self.bytes[off..(off + BS)];
            f(off as u16, buf)?;
            idx = self.dirty.next_index(block);
        }
        Ok(())
    }

    pub(crate) fn is_dirty(&self, addr: u16, len: usize) -> Result<bool, ShadowError> {
        let (sb, eb) = block_span::<TS, BS, BC>(addr, len)?;
        for block in sb..=eb {
            if self.dirty.get(block) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(crate) fn any_dirty(&self) -> bool {
        !self.dirty.is_empty()
    }

    pub(crate) fn clear_all_dirty(&mut self) {
        self.dirty = bitmaps::Bitmap::new();
    }

    pub(crate) fn mark_dirty(&mut self, addr: u16, len: usize) -> Result<(), ShadowError> {
        self.apply_dirty_range(addr, len, true)
    }

    pub(crate) fn clear_dirty(&mut self, addr: u16, len: usize) -> Result<(), ShadowError> {
        self.apply_dirty_range(addr, len, false)
    }

    fn apply_dirty_range(&mut self, addr: u16, len: usize, dirty: bool) -> Result<(), ShadowError> {
        let (sb, eb) = block_span::<TS, BS, BC>(addr, len)?;
        for block in sb..=eb {
            self.dirty.set(block, dirty);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 16-byte table, 4-byte blocks, 4 dirty blocks
    type TestTable = ShadowTable<16, 4, 4>;

    #[test]
    fn new_table_has_no_dirty_blocks() {
        let table: TestTable = ShadowTable::new();
        assert!(!table.is_dirty(0, 16).unwrap());
    }

    #[test]
    fn dirty_tracking_scenarios() {
        // Single block: mark byte 0, block 0 dirty, block 1 clean
        {
            let mut table: TestTable = ShadowTable::new();
            table.mark_dirty(0, 1).unwrap();
            assert!(table.is_dirty(0, 4).unwrap());
            assert!(!table.is_dirty(4, 4).unwrap());
        }

        // Spanning blocks: mark bytes 2-5, blocks 0-1 dirty, block 2 clean
        {
            let mut table: TestTable = ShadowTable::new();
            table.mark_dirty(2, 4).unwrap();
            assert!(table.is_dirty(0, 4).unwrap());
            assert!(table.is_dirty(4, 4).unwrap());
            assert!(!table.is_dirty(8, 4).unwrap());
        }

        // Exact block boundary: mark at addr=4, only block 1 dirty
        {
            let mut table: TestTable = ShadowTable::new();
            table.mark_dirty(4, 4).unwrap();
            assert!(table.is_dirty(4, 4).unwrap());
            assert!(!table.is_dirty(0, 4).unwrap());
            assert!(!table.is_dirty(8, 4).unwrap());
        }

        // Spanning all blocks: mark entire table
        {
            let mut table: TestTable = ShadowTable::new();
            table.mark_dirty(0, 16).unwrap();
            assert!(table.is_dirty(0, 4).unwrap());
            assert!(table.is_dirty(4, 4).unwrap());
            assert!(table.is_dirty(8, 4).unwrap());
            assert!(table.is_dirty(12, 4).unwrap());
        }
    }

    #[test]
    fn is_dirty_zero_len_returns_error() {
        let mut table: TestTable = ShadowTable::new();
        table.mark_dirty(0, 16).unwrap();
        assert_eq!(table.is_dirty(0, 0), Err(ShadowError::ZeroLength));
    }

    #[test]
    fn is_dirty_out_of_bounds_returns_error() {
        let table: TestTable = ShadowTable::new();
        assert_eq!(table.is_dirty(15, 2), Err(ShadowError::OutOfBounds));
        assert_eq!(table.is_dirty(20, 1), Err(ShadowError::OutOfBounds));
    }

    #[test]
    fn is_dirty_query_scenarios() {
        // Test any_dirty and is_dirty with partial block queries
        let mut table: TestTable = ShadowTable::new();

        // Initially: no dirty blocks
        assert!(!table.any_dirty());
        assert!(!table.is_dirty(0, 16).unwrap());

        // Mark only block 1 dirty
        table.mark_dirty(4, 4).unwrap();
        assert!(table.any_dirty());

        // Queries that include block 1 should return dirty
        assert!(table.is_dirty(3, 2).unwrap()); // spans blocks 0-1
        assert!(table.is_dirty(4, 1).unwrap()); // just block 1
        assert!(table.is_dirty(6, 3).unwrap()); // spans blocks 1-2

        // Queries that don't include block 1
        assert!(!table.is_dirty(0, 4).unwrap()); // block 0 only
        assert!(!table.is_dirty(8, 8).unwrap()); // blocks 2-3
    }

    #[test]
    fn with_bytes_errors() {
        let mut table: TestTable = ShadowTable::new();

        // Zero length
        assert_eq!(
            table.with_bytes(0, 0, |_| Ok(())),
            Err(ShadowError::ZeroLength)
        );
        assert_eq!(
            table.with_bytes_mut(0, 0, |_| Ok(())),
            Err(ShadowError::ZeroLength)
        );

        // Out of bounds
        assert_eq!(
            table.with_bytes(14, 4, |_| Ok(())),
            Err(ShadowError::OutOfBounds)
        );
        assert_eq!(
            table.with_bytes_mut(14, 4, |_| Ok(())),
            Err(ShadowError::OutOfBounds)
        );
    }
}
