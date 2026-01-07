use crate::{AddressPolicy, PersistTrigger, ShadowError, table::ShadowTable};

pub struct HostView<'a, const T: usize, const B: usize, const W: usize, P, S>
where
    P: AddressPolicy,
    S: PersistTrigger,
{
    pub(crate) table: &'a mut ShadowTable<T, B, W>,
    pub(crate) policy: &'a P,
    pub(crate) persist: &'a S,
}

impl<'a, const T: usize, const B: usize, const W: usize, P, S> HostView<'a, T, B, W, P, S>
where
    P: AddressPolicy,
    S: PersistTrigger,
{
    pub(crate) fn new(table: &'a mut ShadowTable<T, B, W>, policy: &'a P, persist: &'a S) -> Self {
        Self {
            table,
            policy,
            persist,
        }
    }

    pub fn read_range(&mut self, addr: u16, out: &mut [u8]) -> Result<(), ShadowError> {
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

    /// Expose dirty queries only if you really need them on host side.
    pub fn is_dirty(&self, addr: u16, len: usize) -> Result<bool, ShadowError> {
        self.table.is_dirty(addr, len)
    }

    pub fn any_dirty(&self) -> bool {
        self.table.any_dirty()
    }
}
