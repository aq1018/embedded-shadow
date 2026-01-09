use core::marker::PhantomData;

use crate::{AccessPolicy, PersistTrigger, ShadowError, policy::PersistPolicy, table::ShadowTable};
use bitmaps::{Bits, BitsImpl};

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

    pub fn read_range(&self, addr: u16, out: &mut [u8]) -> Result<(), ShadowError> {
        if !self.access_policy.can_read(addr, out.len()) {
            return Err(ShadowError::Denied);
        }
        self.table.read_range(addr, out)
    }

    /// Host write: marks dirty + may request persistence.
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
