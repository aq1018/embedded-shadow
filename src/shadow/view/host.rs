use core::marker::PhantomData;

use crate::shadow::{
    AccessPolicy, PersistTrigger, ShadowError, WriteResult,
    policy::PersistPolicy,
    slice::{ROSlice, RWSlice, WOSlice},
    table::ShadowTable,
};

/// Application/host-side view of the shadow table.
///
/// Writes through this view mark blocks dirty and may trigger persistence.
/// Reads and writes are subject to the configured access policy.
pub struct HostView<'a, const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK>
where
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
{
    pub(crate) table: &'a mut ShadowTable<TS, BS, BC>,
    pub(crate) access_policy: &'a AP,
    pub(crate) persist_policy: &'a PP,
    pub(crate) persist_trigger: &'a mut PT,
    _phantom: PhantomData<PK>,
}

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK> core::fmt::Debug
    for HostView<'a, TS, BS, BC, AP, PP, PT, PK>
where
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HostView").finish_non_exhaustive()
    }
}

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK>
    HostView<'a, TS, BS, BC, AP, PP, PT, PK>
where
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
{
    pub(crate) fn new(
        table: &'a mut ShadowTable<TS, BS, BC>,
        access_policy: &'a AP,
        persist_policy: &'a PP,
        persist_trigger: &'a mut PT,
    ) -> Self {
        Self {
            table,
            access_policy,
            persist_policy,
            persist_trigger,
            _phantom: PhantomData,
        }
    }

    /// Provides zero-copy read access via ROSlice.
    ///
    /// Returns `Denied` if the access policy rejects the read.
    pub fn with_ro_slice<F, R>(&self, addr: u16, len: usize, f: F) -> Result<R, ShadowError>
    where
        F: FnOnce(ROSlice<'_>) -> R,
    {
        if !self.access_policy.can_read(addr, len) {
            return Err(ShadowError::Denied);
        }
        self.table
            .with_bytes(addr, len, |data| Ok(f(ROSlice::new(data))))
    }

    /// Provides zero-copy write access via WOSlice.
    ///
    /// Returns `Denied` if the access policy rejects the write.
    /// Return `WriteResult::Dirty(result)` from your callback to mark the range as modified.
    /// Return `WriteResult::Clean(result)` to skip dirty marking.
    /// If dirty, triggers persistence based on configured policy.
    pub fn with_wo_slice<F, R>(
        &mut self,
        addr: u16,
        len: usize,
        f: F,
    ) -> Result<WriteResult<R>, ShadowError>
    where
        F: FnOnce(WOSlice<'_>) -> WriteResult<R>,
    {
        if !self.access_policy.can_write(addr, len) {
            return Err(ShadowError::Denied);
        }

        let write_result =
            self.with_bytes_mut_no_persist(addr, len, |data| f(WOSlice::new(data)))?;

        if write_result.is_dirty() {
            let should_persist =
                self.persist_policy
                    .push_persist_keys_for_range(addr, len, |key| {
                        self.persist_trigger.push_key(key)
                    });

            if should_persist {
                self.persist_trigger.request_persist();
            }
        }

        Ok(write_result)
    }

    /// Provides zero-copy read-write access via RWSlice.
    ///
    /// Returns `Denied` if the access policy rejects either read or write.
    /// Return `WriteResult::Dirty(result)` from your callback to mark the range as modified.
    /// Return `WriteResult::Clean(result)` to skip dirty marking.
    /// If dirty, triggers persistence based on configured policy.
    pub fn with_rw_slice<F, R>(
        &mut self,
        addr: u16,
        len: usize,
        f: F,
    ) -> Result<WriteResult<R>, ShadowError>
    where
        F: FnOnce(RWSlice<'_>) -> WriteResult<R>,
    {
        if !self.access_policy.can_read(addr, len) || !self.access_policy.can_write(addr, len) {
            return Err(ShadowError::Denied);
        }

        let write_result =
            self.with_bytes_mut_no_persist(addr, len, |data| f(RWSlice::new(data)))?;

        if write_result.is_dirty() {
            let should_persist =
                self.persist_policy
                    .push_persist_keys_for_range(addr, len, |key| {
                        self.persist_trigger.push_key(key)
                    });

            if should_persist {
                self.persist_trigger.request_persist();
            }
        }

        Ok(write_result)
    }

    pub(crate) fn with_bytes_mut_no_persist<F, R>(
        &mut self,
        addr: u16,
        len: usize,
        f: F,
    ) -> Result<WriteResult<R>, ShadowError>
    where
        F: FnOnce(&mut [u8]) -> WriteResult<R>,
    {
        let write_result = self.table.with_bytes_mut(addr, len, |data| Ok(f(data)))?;

        if write_result.is_dirty() {
            self.table.mark_dirty(addr, len)?;
        }

        Ok(write_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shadow::persist::NoPersist;
    use crate::shadow::policy::NoPersistPolicy;
    use crate::shadow::test_support::{
        DenyAllPolicy, ReadOnlyBelow32, TestHostViewFixture, TestTable, assert_denied,
    };

    #[test]
    fn with_wo_slice_marks_dirty_when_callback_returns_dirty() {
        let mut fixture = TestHostViewFixture::new();

        {
            let mut view = fixture.view();
            let result = view
                .with_wo_slice(0, 4, |mut slice| {
                    slice.copy_from_slice(&[0x01, 0x02, 0x03, 0x04]);
                    WriteResult::Dirty(())
                })
                .unwrap();
            assert!(result.is_dirty());
        }

        assert!(fixture.table.is_dirty(0, 4).unwrap());
    }

    #[test]
    fn with_wo_slice_no_dirty_when_callback_returns_clean() {
        let mut fixture = TestHostViewFixture::new();

        {
            let mut view = fixture.view();
            let result = view
                .with_wo_slice(0, 4, |mut slice| {
                    slice.copy_from_slice(&[0x01, 0x02, 0x03, 0x04]);
                    WriteResult::Clean(()) // Don't mark dirty
                })
                .unwrap();
            assert!(!result.is_dirty());
        }

        assert!(!fixture.table.any_dirty());
    }

    #[test]
    fn slice_access_denied_by_policy() {
        let mut table = TestTable::new();
        let policy = DenyAllPolicy;
        let persist_policy = NoPersistPolicy::default();
        let mut trigger = NoPersist;

        // Test RO denied
        {
            let view = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);
            assert_denied(view.with_ro_slice(0, 4, |_slice| {}));
        }

        // Test WO denied
        {
            let mut view = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);
            assert_denied(view.with_wo_slice(0, 4, |_| WriteResult::Clean(())));
        }

        // Test RW denied
        {
            let mut view = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);
            assert_denied(view.with_rw_slice(0, 4, |_| WriteResult::Clean(())));
        }
    }

    #[test]
    fn with_rw_slice_requires_both_permissions() {
        let mut table = TestTable::new();
        let policy = ReadOnlyBelow32; // Can read anywhere, write only >= 32
        let persist_policy = NoPersistPolicy::default();
        let mut trigger = NoPersist;

        let mut view = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);

        // Below 32: can read but not write, so rw_slice should fail
        assert_denied(view.with_rw_slice(0, 4, |_| WriteResult::Clean(())));

        // At 32: can both read and write, so rw_slice should work
        let result = view.with_rw_slice(32, 4, |_| WriteResult::Dirty(()));
        assert!(result.is_ok());
    }

    #[test]
    fn persist_not_triggered_when_dirty_false() {
        use crate::shadow::policy::AllowAllPolicy;
        use crate::shadow::test_support::{AlwaysPersistPolicy, TrackingPersistTrigger};

        let mut table = TestTable::new();
        let policy = AllowAllPolicy::default();
        let persist_policy = AlwaysPersistPolicy; // Would trigger persist if dirty
        let mut trigger = TrackingPersistTrigger::default();

        {
            let mut view = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);

            // Write data but return Clean to indicate not dirty
            view.with_wo_slice(0, 4, |mut slice| {
                slice.copy_from_slice(&[0x01, 0x02, 0x03, 0x04]);
                WriteResult::Clean(()) // Not dirty - should not trigger persist
            })
            .unwrap();
        }

        // Persist should NOT have been requested
        assert!(!trigger.persist_requested);
        // Table should not be dirty either
        assert!(!table.any_dirty());
    }

    #[test]
    fn wo_slice_triggers_persist_with_always_policy() {
        use crate::shadow::policy::AllowAllPolicy;
        use crate::shadow::test_support::{AlwaysPersistPolicy, TrackingPersistTrigger};

        let mut table = TestTable::new();
        let policy = AllowAllPolicy::default();
        let persist_policy = AlwaysPersistPolicy;
        let mut trigger = TrackingPersistTrigger::default();

        {
            let mut view = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);
            view.with_wo_slice(0, 4, |mut slice| {
                slice.copy_from_slice(&[0x01, 0x02, 0x03, 0x04]);
                WriteResult::Dirty(()) // Mark dirty - should trigger persist
            })
            .unwrap();
        }

        assert!(trigger.persist_requested);
        assert!(table.is_dirty(0, 4).unwrap());
    }

    #[test]
    fn with_rw_slice_marks_dirty_and_triggers_persist() {
        use crate::shadow::policy::AllowAllPolicy;
        use crate::shadow::test_support::{AlwaysPersistPolicy, TrackingPersistTrigger};

        let mut table = TestTable::new();
        let policy = AllowAllPolicy::default();
        let persist_policy = AlwaysPersistPolicy;
        let mut trigger = TrackingPersistTrigger::default();

        {
            let mut view = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);
            let result = view
                .with_rw_slice(0, 4, |mut slice| {
                    slice.copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
                    WriteResult::Dirty(())
                })
                .unwrap();
            assert!(result.is_dirty());
        }

        assert!(trigger.persist_requested);
        assert!(table.is_dirty(0, 4).unwrap());
    }

    #[test]
    fn with_rw_slice_read_then_write() {
        let mut fixture = TestHostViewFixture::new();

        // Pre-populate some data
        {
            let mut view = fixture.view();
            view.with_wo_slice(0, 4, |mut slice| {
                slice.copy_from_slice(&[0x01, 0x02, 0x03, 0x04]);
                WriteResult::Dirty(())
            })
            .unwrap();
        }

        // Clear dirty for next test
        fixture.table.clear_all_dirty();

        // Now read and modify using RW slice
        {
            let mut view = fixture.view();
            let result = view
                .with_rw_slice(0, 4, |mut slice| {
                    let first_byte = slice.read_u8_at(0);
                    slice.copy_from_slice(&[first_byte + 1, 0x22, 0x33, 0x44]);
                    WriteResult::Dirty(first_byte)
                })
                .unwrap();

            assert!(result.is_dirty());
            assert_eq!(result.into_inner(), 0x01);
        }

        // Verify data was modified
        fixture
            .table
            .with_bytes(0, 4, |data| {
                assert_eq!(data, &[0x02, 0x22, 0x33, 0x44]);
                Ok(())
            })
            .unwrap();
        assert!(fixture.table.is_dirty(0, 4).unwrap());
    }
}
