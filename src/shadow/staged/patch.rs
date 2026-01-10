use heapless::Vec;

use crate::shadow::{
    ShadowError,
    types::{StagingBuffer, WriteResult},
};

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

    fn iter_staged<F>(&self, mut f: F) -> Result<(), ShadowError>
    where
        F: FnMut(u16, &[u8]) -> Result<(), ShadowError>,
    {
        for e in self.entries.iter() {
            let buf = &self.data[e.off as usize..(e.off + e.len) as usize];
            f(e.addr, buf)?;
        }
        Ok(())
    }

    fn alloc_staged(
        &mut self,
        addr: u16,
        len: usize,
        f: impl FnOnce(&mut [u8]) -> WriteResult<()>,
    ) -> Result<WriteResult<()>, ShadowError> {
        let off = self.data.len();

        // Pre-allocate space (zero-filled)
        self.data
            .resize(off + len, 0)
            .map_err(|_| ShadowError::StageFull)?;

        // Call user callback - returns WriteResult::Dirty to commit the write
        let result = f(&mut self.data[off..off + len]);

        if result.is_dirty() {
            // Record the entry
            self.entries
                .push(StagedWrite {
                    addr,
                    len: len as u16,
                    off: off as u16,
                })
                .map_err(|_| ShadowError::StageFull)?;
        } else {
            // Reclaim space
            self.data.truncate(off);
        }

        Ok(result)
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
    use crate::shadow::test_support::{TestStage, stage_write};

    #[test]
    fn with_staged_write_accumulates_entries() {
        let mut stage = TestStage::new();

        assert!(!stage.any_staged());

        stage_write(&mut stage, 0, &[0x01, 0x02]).unwrap();
        assert!(stage.any_staged());

        stage_write(&mut stage, 10, &[0x03, 0x04]).unwrap();

        let mut count = 0;
        stage
            .iter_staged(|_, _| {
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
        stage_write(&mut stage, 0, &[0xFF; 60]).unwrap();

        // This should fail - only 4 bytes left but trying to write 8
        assert_eq!(
            stage_write(&mut stage, 100, &[0xAA; 8]),
            Err(ShadowError::StageFull)
        );
    }

    #[test]
    fn stage_full_on_entry_overflow() {
        let mut stage = TestStage::new();

        // Fill all entry slots (8 max)
        for i in 0..8 {
            stage_write(&mut stage, i * 2, &[0x01]).unwrap();
        }

        // 9th entry should fail
        assert_eq!(
            stage_write(&mut stage, 100, &[0x01]),
            Err(ShadowError::StageFull)
        );
    }

    #[test]
    fn clear_staged_empties_buffer() {
        let mut stage = TestStage::new();
        stage_write(&mut stage, 0, &[0x01, 0x02, 0x03]).unwrap();
        stage_write(&mut stage, 10, &[0x04, 0x05]).unwrap();

        assert!(stage.any_staged());

        stage.clear_staged().unwrap();

        assert!(!stage.any_staged());
    }

    #[test]
    fn with_staged_write_commits_when_marked() {
        let mut stage = TestStage::new();

        let result = stage
            .alloc_staged(10, 4, |data| {
                data.copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
                WriteResult::Dirty(())
            })
            .unwrap();

        assert!(result.is_dirty());
        assert!(stage.any_staged());

        // Verify via iter_staged
        let mut found = false;
        stage
            .iter_staged(|addr, data| {
                assert_eq!(addr, 10);
                assert_eq!(data, &[0xAA, 0xBB, 0xCC, 0xDD]);
                found = true;
                Ok(())
            })
            .unwrap();
        assert!(found);
    }

    #[test]
    fn with_staged_write_reclaims_space_when_not_marked() {
        let mut stage = TestStage::new();

        let result = stage
            .alloc_staged(10, 4, |data| {
                data.copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
                WriteResult::Clean(()) // Don't commit the write
            })
            .unwrap();

        assert!(!result.is_dirty());
        assert!(!stage.any_staged());
    }
}
