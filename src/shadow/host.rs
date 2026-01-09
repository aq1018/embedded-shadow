#![allow(unsafe_code)]

use crate::{
    AccessPolicy, HostView, PersistTrigger,
    policy::PersistPolicy,
    storage::{NoStage, ShadowStorageBase, WithStage},
    types::StagingBuffer,
    view::HostViewStaged,
};

pub struct HostShadow<'a, const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK, SS>
where
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    storage: &'a ShadowStorageBase<TS, BS, BC, AP, PP, PT, PK, SS>,
}

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK, SS>
    HostShadow<'a, TS, BS, BC, AP, PP, PT, PK, SS>
where
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    pub(crate) fn new(storage: &'a ShadowStorageBase<TS, BS, BC, AP, PP, PT, PK, SS>) -> Self {
        Self { storage }
    }
}

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK>
    HostShadow<'a, TS, BS, BC, AP, PP, PT, PK, NoStage>
where
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    pub fn with_view<R>(
        &self,
        f: impl FnOnce(&mut HostView<TS, BS, BC, AP, PP, PT, PK>) -> R,
    ) -> R {
        critical_section::with(|_| unsafe { self.with_view_unchecked(f) })
    }

    /// # Safety
    /// This function is unsafe because it requires exclusive access to the shadow's storage.
    /// However, if you can guarantee that the shadow's storage is not being accessed by any
    /// other domains such as ISR or other threads you can safely call this function.
    pub unsafe fn with_view_unchecked<R>(
        &self,
        f: impl FnOnce(&mut HostView<TS, BS, BC, AP, PP, PT, PK>) -> R,
    ) -> R {
        let table = unsafe { &mut *self.storage.table.get() };
        let persist_trigger = unsafe { &mut *self.storage.persist_trigger.get() };
        let access_policy = &self.storage.access_policy;
        let persist_policy = &self.storage.persist_policy;
        let mut view = HostView::new(table, access_policy, persist_policy, persist_trigger);
        f(&mut view)
    }
}

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK, SB>
    HostShadow<'a, TS, BS, BC, AP, PP, PT, PK, WithStage<SB>>
where
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
    SB: StagingBuffer,
{
    pub fn with_view<R>(
        &self,
        f: impl FnOnce(&mut HostViewStaged<TS, BS, BC, AP, PP, PT, PK, SB>) -> R,
    ) -> R {
        critical_section::with(|_| unsafe { self.with_view_unchecked(f) })
    }

    /// # Safety
    /// This function is unsafe because it requires exclusive access to the shadow's storage.
    /// However, if you can guarantee that the shadow's storage is not being accessed by any
    /// other domains such as ISR or other threads you can safely call this function.
    pub unsafe fn with_view_unchecked<R>(
        &self,
        f: impl FnOnce(&mut HostViewStaged<TS, BS, BC, AP, PP, PT, PK, SB>) -> R,
    ) -> R {
        let table = unsafe { &mut *self.storage.table.get() };
        let stage = unsafe { &mut *self.storage.stage_state.get() };
        let persist_trigger = unsafe { &mut *self.storage.persist_trigger.get() };
        let access_policy = &self.storage.access_policy;
        let persist_policy = &self.storage.persist_policy;
        let base = HostView::new(table, access_policy, persist_policy, persist_trigger);
        let mut view = HostViewStaged::new(base, &mut stage.sb);
        f(&mut view)
    }
}
