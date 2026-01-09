use crate::helpers::{block_span, range_span};
use bitmaps::{Bitmap, Bits, BitsImpl};

use crate::error::ShadowError;

pub(crate) struct ShadowTable<const TS: usize, const BS: usize, const BC: usize>
where
    BitsImpl<BC>: Bits,
{
    bytes: [u8; TS],
    dirty: Bitmap<BC>,
}

impl<const TS: usize, const BS: usize, const BC: usize> ShadowTable<TS, BS, BC>
where
    BitsImpl<BC>: Bits,
{
    pub(crate) fn new() -> Self {
        Self {
            bytes: [0; TS],
            dirty: Bitmap::new(),
        }
    }

    fn apply_dirty_range(&mut self, addr: u16, len: usize, dirty: bool) -> Result<(), ShadowError> {
        let (sb, eb) = block_span::<TS, BS, BC>(addr, len)?;
        for block in sb..=eb {
            self.dirty.set(block, dirty);
        }
        Ok(())
    }

    pub(crate) fn read_range(&self, addr: u16, out: &mut [u8]) -> Result<(), ShadowError> {
        let (offset, end) = range_span::<TS>(addr, out.len())?;
        out.copy_from_slice(&self.bytes[offset..end]);
        Ok(())
    }

    pub(crate) fn write_range(&mut self, addr: u16, data: &[u8]) -> Result<(), ShadowError> {
        let (offset, end) = range_span::<TS>(addr, data.len())?;
        self.bytes[offset..end].copy_from_slice(data);

        Ok(())
    }

    pub(crate) fn for_each_dirty_block<F>(&self, mut f: F) -> Result<(), ShadowError>
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

    pub(crate) fn mark_dirty(&mut self, addr: u16, len: usize) -> Result<(), ShadowError> {
        self.apply_dirty_range(addr, len, true)
    }

    pub(crate) fn clear_dirty(&mut self) {
        self.dirty = Bitmap::new();
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
    fn mark_dirty_single_block() {
        let mut table: TestTable = ShadowTable::new();
        table.mark_dirty(0, 1).unwrap();
        assert!(table.is_dirty(0, 1).unwrap());
        assert!(table.is_dirty(0, 4).unwrap()); // whole block 0
        assert!(!table.is_dirty(4, 4).unwrap()); // block 1
    }

    #[test]
    fn mark_dirty_spanning_blocks() {
        let mut table: TestTable = ShadowTable::new();
        // Mark bytes 2-5 dirty (spans blocks 0 and 1)
        table.mark_dirty(2, 4).unwrap();
        assert!(table.is_dirty(0, 4).unwrap()); // block 0
        assert!(table.is_dirty(4, 4).unwrap()); // block 1
        assert!(!table.is_dirty(8, 4).unwrap()); // block 2
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
    fn any_dirty_returns_correct_value() {
        let mut table: TestTable = ShadowTable::new();
        assert!(!table.any_dirty());

        table.mark_dirty(0, 1).unwrap();
        assert!(table.any_dirty());
    }

    #[test]
    fn read_write_range() {
        let mut table: TestTable = ShadowTable::new();
        let data = [1, 2, 3, 4];
        table.write_range(4, &data).unwrap();

        let mut out = [0u8; 4];
        table.read_range(4, &mut out).unwrap();
        assert_eq!(out, data);
    }

    #[test]
    fn read_write_range_errors() {
        let mut table: TestTable = ShadowTable::new();

        // Zero length
        assert_eq!(table.read_range(0, &mut []), Err(ShadowError::ZeroLength));
        assert_eq!(table.write_range(0, &[]), Err(ShadowError::ZeroLength));

        // Out of bounds
        let mut out = [0u8; 4];
        assert_eq!(
            table.read_range(14, &mut out),
            Err(ShadowError::OutOfBounds)
        );
        assert_eq!(
            table.write_range(14, &[1, 2, 3, 4]),
            Err(ShadowError::OutOfBounds)
        );
    }

    #[test]
    fn partial_block_queries() {
        let mut table: TestTable = ShadowTable::new();
        table.mark_dirty(4, 4).unwrap(); // only block 1

        // Queries that include block 1 should return dirty
        assert!(table.is_dirty(3, 2).unwrap()); // spans blocks 0-1
        assert!(table.is_dirty(4, 1).unwrap()); // just block 1
        assert!(table.is_dirty(6, 3).unwrap()); // spans blocks 1-2

        // Queries that don't include block 1
        assert!(!table.is_dirty(0, 4).unwrap()); // block 0 only
        assert!(!table.is_dirty(8, 8).unwrap()); // blocks 2-3
    }
}
