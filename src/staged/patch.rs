use heapless::Vec;

use crate::{ShadowError, types::StagingBuffer};

#[derive(Clone, Copy)]
struct StagedWrite {
    addr: u16,
    len: u16,
    off: u16, // offset into data vec
}

/// Fixed-capacity staging buffer for transactional writes.
///
/// `DC` is the data capacity in bytes, `EC` is the max number of entries.
pub struct PatchStagingBuffer<const DC: usize, const EC: usize> {
    data: Vec<u8, DC>,
    entries: Vec<StagedWrite, EC>,
}

impl<const DC: usize, const EC: usize> PatchStagingBuffer<DC, EC> {
    pub const fn new() -> Self {
        Self {
            data: Vec::new(),
            entries: Vec::new(),
        }
    }

    fn push_bytes(&mut self, bytes: &[u8]) -> Result<u16, ShadowError> {
        let off = self.data.len();
        if off + bytes.len() > DC {
            return Err(ShadowError::StageFull);
        }

        self.data
            .extend_from_slice(bytes)
            .map_err(|_| ShadowError::StageFull)?;

        Ok(off as u16)
    }
}

impl<const DC: usize, const EC: usize> Default for PatchStagingBuffer<DC, EC> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const DC: usize, const EC: usize> StagingBuffer for PatchStagingBuffer<DC, EC> {
    fn any_staged(&self) -> bool {
        !self.entries.is_empty()
    }

    fn for_each_staged<F>(&self, mut f: F) -> Result<(), ShadowError>
    where
        F: FnMut(u16, &[u8]) -> Result<(), ShadowError>,
    {
        for e in self.entries.iter() {
            let buf = &self.data[e.off as usize..(e.off + e.len) as usize];
            f(e.addr, buf)?;
        }
        Ok(())
    }

    fn write_staged(&mut self, addr: u16, data: &[u8]) -> Result<(), ShadowError> {
        let off = self.push_bytes(data)?;

        let entry = StagedWrite {
            addr,
            len: data.len() as u16,
            off,
        };

        self.entries
            .push(entry)
            .map_err(|_| ShadowError::StageFull)?;

        Ok(())
    }

    fn apply_overlay(&self, addr: u16, out: &mut [u8]) -> Result<(), ShadowError> {
        if !self.any_staged() {
            return Ok(());
        }

        // overlay staged writes onto out
        for e in self.entries.iter() {
            let start = e.addr as usize;
            let end = start + e.len as usize;
            let out_start = addr as usize;
            let out_end = out_start + out.len();

            // Check for overlap
            if end <= out_start || start >= out_end {
                continue; // No overlap
            }

            // Calculate overlapping range
            let overlap_start = start.max(out_start);
            let overlap_end = end.min(out_end);

            let data_i = overlap_start - start + e.off as usize;
            let out_i = overlap_start - out_start;
            let n = overlap_end - overlap_start;

            // Write staged data into the output buffer
            out[out_i..out_i + n].copy_from_slice(&self.data[data_i..data_i + n]);
        }

        Ok(())
    }

    fn clear_staged(&mut self) -> Result<(), ShadowError> {
        self.data.clear();
        self.entries.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestStage;

    #[test]
    fn write_staged_accumulates_entries() {
        let mut stage = TestStage::new();

        assert!(!stage.any_staged());

        stage.write_staged(0, &[0x01, 0x02]).unwrap();
        assert!(stage.any_staged());

        stage.write_staged(10, &[0x03, 0x04]).unwrap();

        let mut count = 0;
        stage
            .for_each_staged(|_, _| {
                count += 1;
                Ok(())
            })
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn stage_full_on_data_overflow() {
        let mut stage = TestStage::new();

        // Fill most of the data buffer (64 bytes capacity)
        stage.write_staged(0, &[0xFF; 60]).unwrap();

        // This should fail - only 4 bytes left but trying to write 8
        assert_eq!(
            stage.write_staged(100, &[0xAA; 8]),
            Err(ShadowError::StageFull)
        );
    }

    #[test]
    fn stage_full_on_entry_overflow() {
        let mut stage = TestStage::new();

        // Fill all entry slots (8 max)
        for i in 0..8 {
            stage.write_staged(i * 2, &[0x01]).unwrap();
        }

        // 9th entry should fail
        assert_eq!(
            stage.write_staged(100, &[0x01]),
            Err(ShadowError::StageFull)
        );
    }

    #[test]
    fn clear_staged_empties_buffer() {
        let mut stage = TestStage::new();
        stage.write_staged(0, &[0x01, 0x02, 0x03]).unwrap();
        stage.write_staged(10, &[0x04, 0x05]).unwrap();

        assert!(stage.any_staged());

        stage.clear_staged().unwrap();

        assert!(!stage.any_staged());
    }

    #[test]
    fn apply_overlay_no_overlap_unchanged() {
        let mut stage = TestStage::new();

        // Stage write at address 20-23
        stage.write_staged(20, &[0xAA, 0xBB, 0xCC, 0xDD]).unwrap();

        // Read range 0-3 (no overlap with staged)
        let mut out = [0x11, 0x22, 0x33, 0x44];
        stage.apply_overlay(0, &mut out).unwrap();

        // Output unchanged
        assert_eq!(out, [0x11, 0x22, 0x33, 0x44]);
    }

    #[test]
    fn apply_overlay_full_overlap() {
        let mut stage = TestStage::new();

        // Stage write at address 0-3
        stage.write_staged(0, &[0xAA, 0xBB, 0xCC, 0xDD]).unwrap();

        // Read range 0-3 (full overlap)
        let mut out = [0x00; 4];
        stage.apply_overlay(0, &mut out).unwrap();

        assert_eq!(out, [0xAA, 0xBB, 0xCC, 0xDD]);
    }

    #[test]
    fn apply_overlay_partial_overlap_start() {
        let mut stage = TestStage::new();

        // Stage write at address 4-7
        stage.write_staged(4, &[0xAA, 0xBB, 0xCC, 0xDD]).unwrap();

        // Read range 0-7 (overlaps staged at 4-7)
        let mut out = [0x00; 8];
        stage.apply_overlay(0, &mut out).unwrap();

        // First 4 bytes unchanged, last 4 have staged data
        assert_eq!(out, [0x00, 0x00, 0x00, 0x00, 0xAA, 0xBB, 0xCC, 0xDD]);
    }

    #[test]
    fn apply_overlay_multiple_overlapping_writes() {
        let mut stage = TestStage::new();

        // Stage two overlapping writes at same address
        stage.write_staged(0, &[0x11, 0x22, 0x33, 0x44]).unwrap();
        stage.write_staged(2, &[0xAA, 0xBB]).unwrap(); // Overwrites bytes 2-3

        let mut out = [0x00; 4];
        stage.apply_overlay(0, &mut out).unwrap();

        // Later write wins for overlapping region
        assert_eq!(out, [0x11, 0x22, 0xAA, 0xBB]);
    }
}
