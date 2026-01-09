#![allow(unsafe_code)]

use crate::{AddressPolicy, PersistTrigger, storage::ShadowStorageBase, view::KernelView};

pub struct KernelShadow<'a, const TS: usize, const BS: usize, const BC: usize, AP, PT, SS>
where
    AP: AddressPolicy,
    PT: PersistTrigger,
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    storage: &'a ShadowStorageBase<TS, BS, BC, AP, PT, SS>,
}

impl<'a, const TS: usize, const BS: usize, const BC: usize, AP, PT, SS>
    KernelShadow<'a, TS, BS, BC, AP, PT, SS>
where
    AP: AddressPolicy,
    PT: PersistTrigger,
    bitmaps::BitsImpl<BC>: bitmaps::Bits,
{
    pub(crate) fn new(storage: &'a ShadowStorageBase<TS, BS, BC, AP, PT, SS>) -> Self {
        Self { storage }
    }

    pub fn with_view<R>(&self, f: impl FnOnce(&mut KernelView<TS, BS, BC>) -> R) -> R {
        critical_section::with(|_| self.with_view_unchecked(f))
    }

    pub fn with_view_unchecked<R>(&self, f: impl FnOnce(&mut KernelView<TS, BS, BC>) -> R) -> R {
        let table = unsafe { &mut *self.storage.table.get() };
        let mut view = KernelView::new(table);
        f(&mut view)
    }
}
