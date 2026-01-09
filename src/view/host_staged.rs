use crate::{AccessPolicy, HostView, PersistTrigger, policy::PersistPolicy, types::StagingBuffer};
use bitmaps::{Bits, BitsImpl};

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

    pub fn read_range(&self, addr: u16, out: &mut [u8]) -> Result<(), crate::ShadowError> {
        self.base.read_range(addr, out)
    }

    pub fn write_range(&mut self, addr: u16, data: &[u8]) -> Result<(), crate::ShadowError> {
        self.base.write_range(addr, data)
    }

    pub fn read_range_overlay(&self, addr: u16, out: &mut [u8]) -> Result<(), crate::ShadowError> {
        if !self.base.access_policy.can_read(addr, out.len()) {
            return Err(crate::ShadowError::Denied);
        }

        self.base.read_range(addr, out)?;
        self.sb.apply_overlay(addr, out)?;
        Ok(())
    }

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
