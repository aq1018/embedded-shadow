use crate::{AccessPolicy, HostView, PersistTrigger, policy::PersistPolicy, types::StagingBuffer};
use bitmaps::{Bits, BitsImpl};

/// Host view with transactional staging support.
///
/// Allows writes to be staged and previewed before committing to the shadow table.
pub struct HostViewStaged<'a, const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK, SB>
where
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
    BitsImpl<BC>: Bits,
    SB: StagingBuffer,
{
    base: HostView<'a, TS, BS, BC, AP, PP, PT, PK>,
    sb: &'a mut SB,
}

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK, SB>
    HostViewStaged<'a, TS, BS, BC, AP, PP, PT, PK, SB>
where
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
    BitsImpl<BC>: Bits,
    SB: StagingBuffer,
{
    pub(crate) fn new(base: HostView<'a, TS, BS, BC, AP, PP, PT, PK>, sb: &'a mut SB) -> Self {
        Self { base, sb }
    }

    /// Reads data from the shadow table (ignores staged writes).
    pub fn read_range(&self, addr: u16, out: &mut [u8]) -> Result<(), crate::ShadowError> {
        self.base.read_range(addr, out)
    }

    /// Writes directly to the shadow table, bypassing staging.
    pub fn write_range(&mut self, addr: u16, data: &[u8]) -> Result<(), crate::ShadowError> {
        self.base.write_range(addr, data)
    }

    /// Reads data with staged writes overlaid on top.
    pub fn read_range_overlay(&self, addr: u16, out: &mut [u8]) -> Result<(), crate::ShadowError> {
        if !self.base.access_policy.can_read(addr, out.len()) {
            return Err(crate::ShadowError::Denied);
        }

        self.base.read_range(addr, out)?;
        self.sb.apply_overlay(addr, out)?;
        Ok(())
    }

    /// Stages a write to be applied on commit.
    pub fn write_range_staged(&mut self, addr: u16, data: &[u8]) -> Result<(), crate::ShadowError> {
        if !self.base.access_policy.can_write(addr, data.len()) {
            return Err(crate::ShadowError::Denied);
        }

        self.sb.write_staged(addr, data)
    }

    /// Commits all staged writes to the shadow table.
    ///
    /// Staged writes are applied in order, marking blocks dirty and
    /// triggering persistence as configured. The staging buffer is
    /// cleared after successful commit.
    pub fn commit(&mut self) -> Result<(), crate::ShadowError> {
        if !self.sb.any_staged() {
            return Ok(());
        }

        let mut should_persist = false;
        self.sb.for_each_staged(|addr, data| {
            self.base.write_range_no_persist(addr, data)?;
            should_persist |=
                self.base
                    .persist_policy
                    .push_persist_keys_for_range(addr, data.len(), |key| {
                        self.base.persist_trigger.push_key(key)
                    });
            Ok(())
        })?;

        self.sb.clear_staged()?;

        if should_persist {
            self.base.persist_trigger.request_persist();
        }

        Ok(())
    }

    /// Commits all staged writes to the shadow table.
    #[deprecated(since = "0.1.2", note = "renamed to `commit()`")]
    pub fn action(&mut self) -> Result<(), crate::ShadowError> {
        self.commit()
    }
}

#[cfg(test)]
mod tests {
    use crate::ShadowError;
    use crate::persist::NoPersist;
    use crate::policy::{AllowAllPolicy, NoPersistPolicy};
    use crate::staged::PatchStagingBuffer;
    use crate::test_support::{DenyAllPolicy, TestTable};
    use crate::view::HostView;

    use super::*;

    type TestStage = PatchStagingBuffer<64, 8>;

    #[test]
    fn commit_applies_staged_writes_to_table() {
        let mut table = TestTable::new();
        let policy = AllowAllPolicy::default();
        let persist_policy = NoPersistPolicy::default();
        let mut trigger = NoPersist;
        let mut stage = TestStage::new();

        {
            let base = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);
            let mut view = HostViewStaged::new(base, &mut stage);

            view.write_range_staged(0, &[0xAA, 0xBB, 0xCC, 0xDD])
                .unwrap();
            view.commit().unwrap();
        }

        // Data should be in the table
        let mut buf = [0u8; 4];
        table.read_range(0, &mut buf).unwrap();
        assert_eq!(buf, [0xAA, 0xBB, 0xCC, 0xDD]);
    }

    #[test]
    fn commit_marks_affected_blocks_dirty() {
        let mut table = TestTable::new();
        let policy = AllowAllPolicy::default();
        let persist_policy = NoPersistPolicy::default();
        let mut trigger = NoPersist;
        let mut stage = TestStage::new();

        // Stage data but don't commit yet
        {
            let base = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);
            let mut view = HostViewStaged::new(base, &mut stage);
            view.write_range_staged(0, &[0x01; 4]).unwrap();
        }

        // Not dirty yet (staged only)
        assert!(!table.any_dirty());

        // Now commit
        {
            let base = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);
            let mut view = HostViewStaged::new(base, &mut stage);
            view.commit().unwrap();
        }

        assert!(table.is_dirty(0, 4).unwrap());
    }

    #[test]
    fn commit_clears_staging_buffer() {
        let mut table = TestTable::new();
        let policy = AllowAllPolicy::default();
        let persist_policy = NoPersistPolicy::default();
        let mut trigger = NoPersist;
        let mut stage = TestStage::new();

        {
            let base = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);
            let mut view = HostViewStaged::new(base, &mut stage);

            view.write_range_staged(0, &[0x01; 4]).unwrap();
            view.commit().unwrap();
        }

        assert!(!stage.any_staged());
    }

    #[test]
    fn commit_empty_buffer_is_noop() {
        let mut table = TestTable::new();
        let policy = AllowAllPolicy::default();
        let persist_policy = NoPersistPolicy::default();
        let mut trigger = NoPersist;
        let mut stage = TestStage::new();

        {
            let base = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);
            let mut view = HostViewStaged::new(base, &mut stage);

            // Commit without staging anything
            view.commit().unwrap();
        }

        assert!(!table.any_dirty());
    }

    #[test]
    fn read_range_overlay_shows_staged_data() {
        let mut table = TestTable::new();
        table.write_range(0, &[0x11, 0x22, 0x33, 0x44]).unwrap();

        let policy = AllowAllPolicy::default();
        let persist_policy = NoPersistPolicy::default();
        let mut trigger = NoPersist;
        let mut stage = TestStage::new();

        let base = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);
        let mut view = HostViewStaged::new(base, &mut stage);

        // Stage a write that partially overlaps
        view.write_range_staged(2, &[0xAA, 0xBB]).unwrap();

        let mut buf = [0u8; 4];
        view.read_range_overlay(0, &mut buf).unwrap();

        // First two from table, last two from staged
        assert_eq!(buf, [0x11, 0x22, 0xAA, 0xBB]);
    }

    #[test]
    fn read_range_ignores_staged_data() {
        let mut table = TestTable::new();
        table.write_range(0, &[0x11, 0x22, 0x33, 0x44]).unwrap();

        let policy = AllowAllPolicy::default();
        let persist_policy = NoPersistPolicy::default();
        let mut trigger = NoPersist;
        let mut stage = TestStage::new();

        let base = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);
        let mut view = HostViewStaged::new(base, &mut stage);

        // Stage a write
        view.write_range_staged(0, &[0xAA; 4]).unwrap();

        // Regular read should show table data, not staged
        let mut buf = [0u8; 4];
        view.read_range(0, &mut buf).unwrap();
        assert_eq!(buf, [0x11, 0x22, 0x33, 0x44]);
    }

    #[test]
    fn staged_write_checks_access_policy() {
        let mut table = TestTable::new();
        let policy = DenyAllPolicy;
        let persist_policy = NoPersistPolicy::default();
        let mut trigger = NoPersist;
        let mut stage = TestStage::new();

        let base = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);
        let mut view = HostViewStaged::new(base, &mut stage);

        assert_eq!(
            view.write_range_staged(0, &[0x01; 4]),
            Err(ShadowError::Denied)
        );

        // Nothing should be staged
        assert!(!stage.any_staged());
    }
}
