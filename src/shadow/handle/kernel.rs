#![allow(unsafe_code)]

use crate::shadow::{
    AccessPolicy, PersistTrigger, policy::PersistPolicy, storage::ShadowStorageBase,
    view::KernelView,
};

pub struct KernelShadow<'a, const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK, SS>
where
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    storage: &'a ShadowStorageBase<TS, BS, BC, AP, PP, PT, PK, SS>,
}

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK, SS> core::fmt::Debug
    for KernelShadow<'a, TS, BS, BC, AP, PP, PT, PK, SS>
where
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("KernelShadow").finish_non_exhaustive()
    }
}

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK, SS>
    KernelShadow<'a, TS, BS, BC, AP, PP, PT, PK, SS>
where
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    pub(crate) fn new(storage: &'a ShadowStorageBase<TS, BS, BC, AP, PP, PT, PK, SS>) -> Self {
        Self { storage }
    }

    pub fn with_view<R>(&self, f: impl FnOnce(&mut KernelView<TS, BS, BC>) -> R) -> R {
        critical_section::with(|_| unsafe { self.with_view_unchecked(f) })
    }

    /// # Safety
    /// This function is unsafe because it requires exclusive access to the ShadowStorage.
    /// You must ensure that no other code is accessing the ShadowStorage at the same time.
    /// Generally, if your kernel is running inside an ISR and cannot be interrupted by other ISRs,
    /// then it is safe to call this function.
    pub unsafe fn with_view_unchecked<R>(
        &self,
        f: impl FnOnce(&mut KernelView<TS, BS, BC>) -> R,
    ) -> R {
        let table = unsafe { &mut *self.storage.table.get() };
        let mut view = KernelView::new(table);
        f(&mut view)
    }
}
