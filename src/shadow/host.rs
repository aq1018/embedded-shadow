#![allow(unsafe_code)]

use crate::{
    AddressPolicy, HostView, PersistTrigger,
    storage::{NoStage, ShadowStorageBase, WithStage},
    types::StagingBuffer,
    view::HostViewStaged,
};

pub struct HostShadow<'a, const TS: usize, const BS: usize, const BC: usize, AP, PT, SS>
where
    AP: AddressPolicy,
    PT: PersistTrigger,
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    storage: &'a ShadowStorageBase<TS, BS, BC, AP, PT, SS>,
}

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PT, SS>
    HostShadow<'a, TS, BS, BC, AP, PT, SS>
where
    AP: AddressPolicy,
    PT: PersistTrigger,
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    pub(crate) fn new(storage: &'a ShadowStorageBase<TS, BS, BC, AP, PT, SS>) -> Self {
        Self { storage }
    }
}

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PT>
    HostShadow<'a, TS, BS, BC, AP, PT, NoStage>
where
    AP: AddressPolicy,
    PT: PersistTrigger,
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    pub fn with_view<R>(&self, f: impl FnOnce(&mut HostView<TS, BS, BC, AP, PT>) -> R) -> R {
        critical_section::with(|_| self.with_view_unchecked(f))
    }

    pub fn with_view_unchecked<R>(
        &self,
        f: impl FnOnce(&mut HostView<TS, BS, BC, AP, PT>) -> R,
    ) -> R {
        let table = unsafe { &mut *self.storage.table.get() };
        let policy = &self.storage.policy;
        let persist = &self.storage.persist;
        let mut view = HostView::new(table, policy, persist);
        f(&mut view)
    }
}

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PT, SB>
    HostShadow<'a, TS, BS, BC, AP, PT, WithStage<SB>>
where
    AP: AddressPolicy,
    PT: PersistTrigger,
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
    SB: StagingBuffer,
{
    pub fn with_view<R>(
        &self,
        f: impl FnOnce(&mut HostViewStaged<TS, BS, BC, AP, PT, SB>) -> R,
    ) -> R {
        critical_section::with(|_| self.with_view_unchecked(f))
    }

    pub fn with_view_unchecked<R>(
        &self,
        f: impl FnOnce(&mut HostViewStaged<TS, BS, BC, AP, PT, SB>) -> R,
    ) -> R {
        let table = unsafe { &mut *self.storage.table.get() };
        let stage = unsafe { &mut *self.storage.stage.get() };
        let policy = &self.storage.policy;
        let persist = &self.storage.persist;
        let base = HostView::new(table, policy, persist);
        let mut view = HostViewStaged::new(base, &mut stage.sb);
        f(&mut view)
    }
}
