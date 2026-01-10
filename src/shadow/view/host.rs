use core::marker::PhantomData;

use crate::{AccessPolicy, PersistTrigger, ShadowError, policy::PersistPolicy, table::ShadowTable};
use bitmaps::{Bits, BitsImpl};

/// Application/host-side view of the shadow table.
///
/// Writes through this view mark blocks dirty and may trigger persistence.
/// Reads and writes are subject to the configured access policy.
pub struct HostView<'a, const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK>
where
    BitsImpl<BC>: Bits,
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

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK>
    HostView<'a, TS, BS, BC, AP, PP, PT, PK>
where
    BitsImpl<BC>: Bits,
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

    /// Reads data from the shadow table.
    ///
    /// Returns `Denied` if the access policy rejects the read.
    pub fn read_range(&self, addr: u16, out: &mut [u8]) -> Result<(), ShadowError> {
        if !self.access_policy.can_read(addr, out.len()) {
            return Err(ShadowError::Denied);
        }
        self.table.read_range(addr, out)
    }

    /// Writes data to the shadow table, marking blocks dirty.
    ///
    /// May trigger persistence based on the configured policy.
    pub fn write_range(&mut self, addr: u16, data: &[u8]) -> Result<(), ShadowError> {
        self.write_range_no_persist(addr, data)?;

        let should_persist =
            self.persist_policy
                .push_persist_keys_for_range(addr, data.len(), |key| {
                    self.persist_trigger.push_key(key)
                });

        if should_persist {
            self.persist_trigger.request_persist();
        }

        Ok(())
    }

    pub(crate) fn write_range_no_persist(
        &mut self,
        addr: u16,
        data: &[u8],
    ) -> Result<(), ShadowError> {
        if !self.access_policy.can_write(addr, data.len()) {
            return Err(ShadowError::Denied);
        }

        self.table.write_range(addr, data)?;
        self.table.mark_dirty(addr, data.len())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persist::NoPersist;
    use crate::policy::{AllowAllPolicy, NoPersistPolicy};
    use crate::shadow::test_support::{DenyAllPolicy, ReadOnlyBelow32, TestTable};

    #[test]
    fn host_write_marks_dirty() {
        let mut table = TestTable::new();
        let policy = AllowAllPolicy::default();
        let persist_policy = NoPersistPolicy::default();
        let mut trigger = NoPersist;

        {
            let mut view = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);
            view.write_range(0, &[0x01, 0x02, 0x03, 0x04]).unwrap();
        }

        assert!(table.is_dirty(0, 4).unwrap());
    }

    #[test]
    fn host_write_spanning_blocks_marks_all_dirty() {
        let mut table = TestTable::new();
        let policy = AllowAllPolicy::default();
        let persist_policy = NoPersistPolicy::default();
        let mut trigger = NoPersist;

        {
            let mut view = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);
            // Write spans blocks 0 and 1 (bytes 8-23)
            view.write_range(8, &[0xAA; 16]).unwrap();
        }

        // Both block 0 and block 1 should be dirty
        assert!(table.is_dirty(0, 16).unwrap()); // block 0
        assert!(table.is_dirty(16, 16).unwrap()); // block 1
    }

    #[test]
    fn host_read_does_not_mark_dirty() {
        let mut table = TestTable::new();
        let policy = AllowAllPolicy::default();
        let persist_policy = NoPersistPolicy::default();
        let mut trigger = NoPersist;

        let view = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);

        let mut buf = [0u8; 4];
        view.read_range(0, &mut buf).unwrap();

        assert!(!table.any_dirty());
    }

    #[test]
    fn read_denied_returns_error() {
        let mut table = TestTable::new();
        let policy = DenyAllPolicy;
        let persist_policy = NoPersistPolicy::default();
        let mut trigger = NoPersist;

        let view = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);
        let mut buf = [0u8; 4];

        assert_eq!(view.read_range(0, &mut buf), Err(ShadowError::Denied));
    }

    #[test]
    fn write_denied_returns_error() {
        let mut table = TestTable::new();
        let policy = DenyAllPolicy;
        let persist_policy = NoPersistPolicy::default();
        let mut trigger = NoPersist;

        let mut view = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);

        assert_eq!(view.write_range(0, &[0x01, 0x02]), Err(ShadowError::Denied));
    }

    #[test]
    fn denied_write_does_not_modify_state() {
        let mut table = TestTable::new();
        table.write_range(0, &[0xFF; 4]).unwrap();

        let policy = DenyAllPolicy;
        let persist_policy = NoPersistPolicy::default();
        let mut trigger = NoPersist;

        {
            let mut view = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);
            let _ = view.write_range(0, &[0x00; 4]); // Should fail
        }

        // Original data unchanged
        let mut buf = [0u8; 4];
        table.read_range(0, &mut buf).unwrap();
        assert_eq!(buf, [0xFF; 4]);

        // No dirty bits set
        assert!(!table.any_dirty());
    }

    #[test]
    fn denied_read_does_not_leak_data() {
        let mut table = TestTable::new();
        table.write_range(0, &[0xAA; 4]).unwrap();

        let policy = DenyAllPolicy;
        let persist_policy = NoPersistPolicy::default();
        let mut trigger = NoPersist;

        let view = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);

        let mut buf = [0x00u8; 4];
        let _ = view.read_range(0, &mut buf); // Should fail

        // Buffer unchanged (no data leaked)
        assert_eq!(buf, [0x00; 4]);
    }

    #[test]
    fn partial_policy_allows_permitted_ranges() {
        let mut table = TestTable::new();
        let policy = ReadOnlyBelow32;
        let persist_policy = NoPersistPolicy::default();
        let mut trigger = NoPersist;

        let mut view = HostView::new(&mut table, &policy, &persist_policy, &mut trigger);

        // Read should work anywhere
        let mut buf = [0u8; 4];
        assert!(view.read_range(0, &mut buf).is_ok());
        assert!(view.read_range(32, &mut buf).is_ok());

        // Write below 32 should fail
        assert_eq!(view.write_range(0, &[0x01; 4]), Err(ShadowError::Denied));

        // Write at 32 and above should work
        assert!(view.write_range(32, &[0x01; 4]).is_ok());
    }
}
