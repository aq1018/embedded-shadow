use crate::{AddressPolicy, PersistTrigger, ShadowError, table::ShadowTable};
use bitmaps::{Bits, BitsImpl};

pub struct HostView<'a, const TS: usize, const BS: usize, const BC: usize, AP, PT>
where
    BitsImpl<BC>: Bits,
    AP: AddressPolicy,
    PT: PersistTrigger,
{
    pub(crate) table: &'a mut ShadowTable<TS, BS, BC>,
    pub(crate) policy: &'a AP,
    pub(crate) persist: &'a PT,
}

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PT> HostView<'a, TS, BS, BC, AP, PT>
where
    BitsImpl<BC>: Bits,
    AP: AddressPolicy,
    PT: PersistTrigger,
{
    pub(crate) fn new(
        table: &'a mut ShadowTable<TS, BS, BC>,
        policy: &'a AP,
        persist: &'a PT,
    ) -> Self {
        Self {
            table,
            policy,
            persist,
        }
    }
}

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PT> HostView<'a, TS, BS, BC, AP, PT>
where
    BitsImpl<BC>: Bits,
    AP: AddressPolicy,
    PT: PersistTrigger,
{
    pub fn read_range(&self, addr: u16, out: &mut [u8]) -> Result<(), ShadowError> {
        if !self.policy.can_read(addr, out.len()) {
            return Err(ShadowError::Denied);
        }
        self.table.read_range(addr, out)
    }

    /// Host write: marks dirty + may request persistence.
    pub fn write_range(&mut self, addr: u16, data: &[u8]) -> Result<(), ShadowError> {
        if !self.policy.can_write(addr, data.len()) {
            return Err(ShadowError::Denied);
        }

        self.table.write_range(addr, data)?;
        self.table.mark_dirty(addr, data.len())?;

        if self.policy.triggers_persist(addr, data.len()) {
            self.persist.request_persist();
        }

        Ok(())
    }
}
