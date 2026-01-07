use crate::error::ShadowError;

#[derive(Clone, Copy)]
struct WordMaskSpan {
    sw: usize,
    ew: usize,
    sb: usize,
    eb: usize,
}

pub(crate) struct ShadowTable<const T: usize, const B: usize, const W: usize> {
    bytes: [u8; T],
    dirty_words: [u32; W],
}

impl<const T: usize, const B: usize, const W: usize> ShadowTable<T, B, W> {
    pub(crate) const DIRTY_CAPACITY_BYTES: usize = W * 32 * B;

    pub(crate) fn new() -> Self {
        debug_assert!(T != 0);
        debug_assert!(B != 0);
        debug_assert!(W != 0);
        debug_assert!(T <= Self::DIRTY_CAPACITY_BYTES);

        Self {
            bytes: [0; T],
            dirty_words: [0; W],
        }
    }

    pub(crate) fn read_range(&self, addr: u16, out: &mut [u8]) -> Result<(), ShadowError> {
        let (offset, end) = self.range_span(addr, out.len())?;
        out.copy_from_slice(&self.bytes[offset..end]);
        Ok(())
    }

    pub(crate) fn write_range(&mut self, addr: u16, data: &[u8]) -> Result<(), ShadowError> {
        let (offset, end) = self.range_span(addr, data.len())?;
        self.bytes[offset..end].copy_from_slice(data);

        Ok(())
    }

    pub(crate) fn mark_dirty(&mut self, addr: u16, len: usize) -> Result<(), ShadowError> {
        let s = self.word_mask_span(addr, len)?;
        self.apply_masks_mut(s, |w, mask| *w |= mask);
        Ok(())
    }

    pub(crate) fn clear_dirty(&mut self, addr: u16, len: usize) -> Result<(), ShadowError> {
        let s = self.word_mask_span(addr, len)?;
        self.apply_masks_mut(s, |w, mask| *w &= !mask);
        Ok(())
    }

    pub(crate) fn is_dirty(&self, addr: u16, len: usize) -> Result<bool, ShadowError> {
        let s = self.word_mask_span(addr, len)?;
        Ok(self.apply_masks_any(s, |w, mask| (w & mask) != 0))
    }

    pub(crate) fn any_dirty(&self) -> bool {
        self.dirty_words.iter().any(|&w| w != 0)
    }

    fn apply_masks_mut(&mut self, s: WordMaskSpan, mut f: impl FnMut(&mut u32, u32)) {
        if s.sw == s.ew {
            let mask = mask_in_word(s.sb, s.eb);
            f(&mut self.dirty_words[s.sw], mask);
            return;
        }

        // first partial word
        let first_mask = mask_in_word(s.sb, s.sw * 32 + 31);
        f(&mut self.dirty_words[s.sw], first_mask);

        // middle full words
        for wi in (s.sw + 1)..s.ew {
            f(&mut self.dirty_words[wi], u32::MAX);
        }

        // last partial word
        let last_mask = mask_in_word(s.ew * 32, s.eb);
        f(&mut self.dirty_words[s.ew], last_mask);
    }

    fn apply_masks_any(&self, s: WordMaskSpan, mut pred: impl FnMut(u32, u32) -> bool) -> bool {
        if s.sw == s.ew {
            let mask = mask_in_word(s.sb, s.eb);
            return pred(self.dirty_words[s.sw], mask);
        }

        let first_mask = mask_in_word(s.sb, s.sw * 32 + 31);
        if pred(self.dirty_words[s.sw], first_mask) {
            return true;
        }

        for wi in (s.sw + 1)..s.ew {
            if pred(self.dirty_words[wi], u32::MAX) {
                return true;
            }
        }

        let last_mask = mask_in_word(s.ew * 32, s.eb);
        pred(self.dirty_words[s.ew], last_mask)
    }

    fn word_mask_span(&self, addr: u16, len: usize) -> Result<WordMaskSpan, ShadowError> {
        let (sb, eb) = self.block_span(addr, len)?;
        let sw = sb / 32;
        let ew = eb / 32;

        if ew >= W {
            return Err(ShadowError::OutOfBounds);
        }

        Ok(WordMaskSpan { sw, ew, sb, eb })
    }

    fn block_span(&self, addr: u16, len: usize) -> Result<(usize, usize), ShadowError> {
        let (offset, end) = self.range_span(addr, len)?;
        let sb = offset / B;
        let eb = (end - 1) / B; // inclusive

        if eb >= W * 32 {
            return Err(ShadowError::OutOfBounds);
        }

        Ok((sb, eb))
    }

    fn range_span(&self, addr: u16, len: usize) -> Result<(usize, usize), ShadowError> {
        if len == 0 {
            return Err(ShadowError::ZeroLength);
        }

        let offset = addr as usize;
        let end = offset.checked_add(len).ok_or(ShadowError::OutOfBounds)?;

        if end > T {
            return Err(ShadowError::OutOfBounds);
        }

        Ok((offset, end))
    }
}

fn mask_in_word(sb: usize, eb: usize) -> u32 {
    let lo = (sb % 32) as u32;
    let hi = (eb % 32) as u32;
    let width = hi - lo + 1;
    ((1u32 << width) - 1) << lo
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
    fn clear_dirty_single_block() {
        let mut table: TestTable = ShadowTable::new();
        table.mark_dirty(0, 16).unwrap(); // mark all dirty
        table.clear_dirty(0, 4).unwrap(); // clear block 0
        assert!(!table.is_dirty(0, 4).unwrap());
        assert!(table.is_dirty(4, 4).unwrap()); // block 1 still dirty
    }

    #[test]
    fn clear_dirty_spanning_blocks() {
        let mut table: TestTable = ShadowTable::new();
        table.mark_dirty(0, 16).unwrap();
        table.clear_dirty(2, 6).unwrap(); // spans blocks 0 and 1
        assert!(!table.is_dirty(0, 8).unwrap()); // blocks 0 and 1 cleared
        assert!(table.is_dirty(8, 4).unwrap()); // block 2 still dirty
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
    fn block_span_edge_cases() {
        let table: TestTable = ShadowTable::new();

        // Zero length
        assert_eq!(table.block_span(0, 0), Err(ShadowError::ZeroLength));

        // Out of bounds
        assert_eq!(table.block_span(15, 2), Err(ShadowError::OutOfBounds));

        // Single byte at block boundary
        assert_eq!(table.block_span(4, 1), Ok((1, 1)));

        // Exact block
        assert_eq!(table.block_span(4, 4), Ok((1, 1)));

        // Spanning all blocks
        assert_eq!(table.block_span(0, 16), Ok((0, 3)));

        // Last byte of table
        assert_eq!(table.block_span(15, 1), Ok((3, 3)));
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
    fn mark_and_clear_idempotent() {
        let mut table: TestTable = ShadowTable::new();

        // Double mark
        table.mark_dirty(0, 4).unwrap();
        table.mark_dirty(0, 4).unwrap();
        assert!(table.is_dirty(0, 4).unwrap());

        // Double clear
        table.clear_dirty(0, 4).unwrap();
        table.clear_dirty(0, 4).unwrap();
        assert!(!table.is_dirty(0, 4).unwrap());
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
