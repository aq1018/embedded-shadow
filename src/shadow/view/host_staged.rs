use crate::shadow::{
    AccessPolicy, HostView, PersistTrigger, ShadowError,
    policy::PersistPolicy,
    slice::{ROSlice, RWSlice, WOSlice},
    types::StagingBuffer,
};

/// Host view with transactional staging support.
///
/// Allows writes to be staged and previewed before committing to the shadow table.
pub struct HostViewStaged<'a, const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK, SB>
where
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
    SB: StagingBuffer,
{
    base: HostView<'a, TS, BS, BC, AP, PP, PT, PK>,
    sb: &'a mut SB,
}

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK, SB> core::fmt::Debug
    for HostViewStaged<'a, TS, BS, BC, AP, PP, PT, PK, SB>
where
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
    SB: StagingBuffer,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HostViewStaged").finish_non_exhaustive()
    }
}

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK, SB>
    HostViewStaged<'a, TS, BS, BC, AP, PP, PT, PK, SB>
where
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
    SB: StagingBuffer,
{
    pub(crate) fn new(base: HostView<'a, TS, BS, BC, AP, PP, PT, PK>, sb: &'a mut SB) -> Self {
        Self { base, sb }
    }

    /// Provides zero-copy read access via ROSlice (ignores staged writes).
    pub fn with_ro_slice<F, R>(&self, addr: u16, len: usize, f: F) -> Result<R, ShadowError>
    where
        F: FnOnce(ROSlice<'_>) -> R,
    {
        self.base.with_ro_slice(addr, len, f)
    }

    /// Provides zero-copy write access via WOSlice, bypassing staging.
    pub fn with_wo_slice<F, R>(
        &mut self,
        addr: u16,
        len: usize,
        f: F,
    ) -> Result<(bool, R), ShadowError>
    where
        F: FnOnce(WOSlice<'_>) -> (bool, R),
    {
        self.base.with_wo_slice(addr, len, f)
    }

    /// Provides zero-copy read-write access via RWSlice, bypassing staging.
    pub fn with_rw_slice<F, R>(
        &mut self,
        addr: u16,
        len: usize,
        f: F,
    ) -> Result<(bool, R), ShadowError>
    where
        F: FnOnce(RWSlice<'_>) -> (bool, R),
    {
        self.base.with_rw_slice(addr, len, f)
    }

    /// Zero-copy staged write access via RWSlice.
    ///
    /// Return `(true, result)` from your callback to commit the staged write.
    /// If you return `false`, no data is staged and space is reclaimed.
    pub fn alloc_staged<F, R>(
        &mut self,
        addr: u16,
        len: usize,
        f: F,
    ) -> Result<(bool, R), ShadowError>
    where
        F: FnOnce(RWSlice<'_>) -> (bool, R),
    {
        if !self.base.access_policy.can_write(addr, len) {
            return Err(ShadowError::Denied);
        }

        let mut result = None;
        let written = self.sb.alloc_staged(addr, len, |data| {
            let (written, r) = f(RWSlice::new(data));
            result = Some(r);
            written
        })?;

        Ok((written, result.unwrap()))
    }

    /// Commits all staged writes to the shadow table.
    ///
    /// Staged writes are applied in order, marking blocks dirty and
    /// triggering persistence as configured. The staging buffer is
    /// cleared after successful commit.
    pub fn commit_staged(&mut self) -> Result<(), ShadowError> {
        if !self.sb.any_staged() {
            return Ok(());
        }

        let mut should_persist = false;
        self.sb.iter_staged(|addr, data| {
            self.base
                .with_bytes_mut_no_persist(addr, data.len(), |buf| {
                    buf.copy_from_slice(data);
                    (true, ())
                })?;
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

    /// Returns true if there are any staged writes pending.
    pub fn is_staged(&self) -> bool {
        self.sb.any_staged()
    }

    /// Iterates over each staged write, providing its address and data as ROSlice.
    pub fn iter_staged<F>(&self, mut f: F) -> Result<(), ShadowError>
    where
        F: FnMut(u16, ROSlice<'_>) -> Result<(), ShadowError>,
    {
        self.sb
            .iter_staged(|addr, data| f(addr, ROSlice::new(data)))
    }

    /// Clears all staged writes without committing them.
    pub fn clear_staged(&mut self) -> Result<(), ShadowError> {
        self.sb.clear_staged()
    }
}

#[cfg(test)]
mod tests {
    use crate::shadow::persist::NoPersist;
    use crate::shadow::policy::NoPersistPolicy;
    use crate::shadow::test_support::{
        DenyAllPolicy, TestHostViewStagedFixture, TestStage, TestTable, assert_denied,
        assert_table_bytes,
    };
    use crate::shadow::view::HostView;

    use super::*;

    #[test]
    fn commit_applies_staged_writes_to_table() {
        let mut fixture = TestHostViewStagedFixture::new();

        {
            let mut view = fixture.view();
            view.alloc_staged(0, 4, |mut slice| {
                slice.copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
                (true, ())
            })
            .unwrap();
            view.commit_staged().unwrap();
        }

        assert_table_bytes(&fixture.table, 0, &[0xAA, 0xBB, 0xCC, 0xDD]);
    }

    #[test]
    fn commit_marks_affected_blocks_dirty() {
        let mut fixture = TestHostViewStagedFixture::new();

        {
            let mut view = fixture.view();
            view.alloc_staged(0, 4, |mut slice| {
                slice.copy_from_slice(&[0x01; 4]);
                (true, ())
            })
            .unwrap();
        }

        // Not dirty yet (staged only)
        assert!(!fixture.table.any_dirty());

        {
            let mut view = fixture.view();
            view.commit_staged().unwrap();
        }

        assert!(fixture.table.is_dirty(0, 4).unwrap());
    }

    #[test]
    fn alloc_staged_writes_to_staging_buffer() {
        let mut fixture = TestHostViewStagedFixture::new();

        {
            let mut view = fixture.view();

            let (written, _) = view
                .alloc_staged(0, 4, |mut slice| {
                    slice.copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
                    (true, ())
                })
                .unwrap();

            assert!(written);
            view.commit_staged().unwrap();
        }

        assert!(!fixture.stage.any_staged());
        assert_table_bytes(&fixture.table, 0, &[0xAA, 0xBB, 0xCC, 0xDD]);
    }

    #[test]
    fn alloc_staged_no_write_reclaims_space() {
        let mut fixture = TestHostViewStagedFixture::new();
        let mut view = fixture.view();

        let (written, _) = view
            .alloc_staged(0, 4, |mut slice| {
                slice.copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
                (false, ()) // Don't commit the write
            })
            .unwrap();

        assert!(!written);
        assert!(!fixture.stage.any_staged());
    }

    #[test]
    fn alloc_staged_checks_access_policy() {
        let mut table = TestTable::new();
        let policy = DenyAllPolicy;
        let persist_policy = NoPersistPolicy::default();
        let mut trigger = NoPersist;
        let mut stage = TestStage::new();

        let base = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);
        let mut view = HostViewStaged::new(base, &mut stage);

        assert_denied(view.alloc_staged(0, 4, |_| (false, ())));
        assert!(!stage.any_staged());
    }

    #[test]
    fn commit_error_leaves_staging_intact() {
        use crate::shadow::test_support::stage_write;

        let mut fixture = TestHostViewStagedFixture::new();

        // Stage a valid write
        stage_write(&mut fixture.stage, 0, &[0x11, 0x22]).unwrap();
        // Stage an out-of-bounds write (addr 100 is past the 64-byte table)
        stage_write(&mut fixture.stage, 100, &[0xAA, 0xBB]).unwrap();

        assert!(fixture.stage.any_staged());

        // Try to commit - should fail on the out-of-bounds write
        let mut view = fixture.view();
        let result = view.commit_staged();
        assert!(result.is_err());

        // Staging buffer should still have entries (not cleared on error)
        assert!(fixture.stage.any_staged());
    }

    #[test]
    fn commit_staged_triggers_persist() {
        use crate::shadow::policy::AllowAllPolicy;
        use crate::shadow::staged::PatchStagingBuffer;
        use crate::shadow::table::ShadowTable;
        use crate::shadow::test_support::{AlwaysPersistPolicy, TrackingPersistTrigger};
        use crate::shadow::view::HostView;

        let mut table: ShadowTable<64, 16, 4> = ShadowTable::new();
        let policy = AllowAllPolicy::default();
        let persist_policy = AlwaysPersistPolicy;
        let mut trigger = TrackingPersistTrigger::default();
        let mut stage: PatchStagingBuffer<64, 8> = PatchStagingBuffer::new();

        {
            let base = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);
            let mut view = HostViewStaged::new(base, &mut stage);

            // Stage a write
            view.alloc_staged(0, 4, |mut slice| {
                slice.copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
                (true, ())
            })
            .unwrap();

            // Commit the staged write
            view.commit_staged().unwrap();
        }

        // Persistence should have been triggered
        assert!(trigger.persist_requested);
        // Table should be dirty
        assert!(table.is_dirty(0, 4).unwrap());
    }
}
