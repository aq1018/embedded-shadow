use crate::staged::internal::StagedImpl;
use crate::{AddressPolicy, HostView, PersistTrigger};

pub struct HostViewStaged<'a, const T: usize, const B: usize, const W: usize, P, S, St>
where
    P: AddressPolicy,
    S: PersistTrigger,
    St: StagedImpl<T, B, W>,
{
    base: HostView<'a, T, B, W, P, S>,
    staged: &'a mut St,
}

impl<'a, const T: usize, const B: usize, const W: usize, P, S, St>
    HostViewStaged<'a, T, B, W, P, S, St>
where
    P: AddressPolicy,
    S: PersistTrigger,
    St: StagedImpl<T, B, W>,
{
    pub(crate) fn new(base: HostView<'a, T, B, W, P, S>, staged: &'a mut St) -> Self {
        Self { base, staged }
    }

    pub fn read_range(&mut self, addr: u16, out: &mut [u8]) -> Result<(), crate::ShadowError> {
        self.base.read_range(addr, out)
    }

    pub fn write_range(&mut self, addr: u16, data: &[u8]) -> Result<(), crate::ShadowError> {
        if !self.base.policy.can_write(addr, data.len()) {
            return Err(crate::ShadowError::Denied);
        }

        if self.base.policy.stage_only(addr, data.len()) {
            return self
                .staged
                .write_range_staged(&*self.base.table, addr, data);
        }

        self.base.write_range(addr, data)
    }

    pub fn read_range_overlay(
        &mut self,
        addr: u16,
        out: &mut [u8],
    ) -> Result<(), crate::ShadowError> {
        if !self.base.policy.can_read(addr, out.len()) {
            return Err(crate::ShadowError::Denied);
        }
        self.staged.read_range_overlay(&*self.base.table, addr, out)
    }

    pub fn action(&mut self) -> Result<(), crate::ShadowError> {
        let needs = self.staged.action(self.base.table, self.base.policy)?;
        if needs {
            self.base.persist.request_persist();
        }
        Ok(())
    }

    pub fn has_staged(&self) -> bool {
        self.staged.has_staged()
    }
}
