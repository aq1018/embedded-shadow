#![allow(unsafe_code)]

use core::cell::UnsafeCell;

use crate::{
    persist::PersistTrigger,
    policy::AddressPolicy,
    table::ShadowTable,
    view::{HostView, KernelView},
};

pub struct NoStage;

pub struct WithStage<St> {
    pub(crate) staged: St,
}

pub struct ShadowStorageBase<
    const T: usize,
    const B: usize,
    const W: usize,
    P: AddressPolicy,
    S: PersistTrigger,
    SS,
> {
    table: UnsafeCell<ShadowTable<T, B, W>>,
    policy: P,
    persist: S,
    stage: UnsafeCell<SS>,
}

unsafe impl<const T: usize, const B: usize, const W: usize, P, S, SS> Sync
    for ShadowStorageBase<T, B, W, P, S, SS>
where
    P: AddressPolicy,
    S: PersistTrigger,
{
}

pub type ShadowStorage<const T: usize, const B: usize, const W: usize, P, S> =
    ShadowStorageBase<T, B, W, P, S, NoStage>;

impl<const T: usize, const B: usize, const W: usize, P, S> ShadowStorageBase<T, B, W, P, S, NoStage>
where
    P: AddressPolicy,
    S: PersistTrigger,
{
    pub fn new(policy: P, persist: S) -> Self {
        Self {
            table: UnsafeCell::new(ShadowTable::new()),
            policy,
            persist,
            stage: UnsafeCell::new(NoStage),
        }
    }

    /// Upgrade this storage to staged mode by supplying a staging implementation.
    pub fn with_staging<St>(self, staged: St) -> ShadowStorageBase<T, B, W, P, S, WithStage<St>> {
        ShadowStorageBase {
            table: self.table,
            policy: self.policy,
            persist: self.persist,
            stage: UnsafeCell::new(WithStage { staged }),
        }
    }
}

impl<const T: usize, const B: usize, const W: usize, P, S, SS> ShadowStorageBase<T, B, W, P, S, SS>
where
    P: AddressPolicy,
    S: PersistTrigger,
{
    pub fn host_with_view<R>(
        &self,
        f: impl for<'a> FnOnce(&mut HostView<'a, T, B, W, P, S>) -> R,
    ) -> R {
        critical_section::with(|_cs| {
            // SAFETY: critical section ensures exclusive access to the table for duration of closure.
            unsafe { self.host_with_view_unchecked(f) }
        })
    }

    pub fn kernel_with_view<R>(
        &self,
        f: impl for<'a> FnOnce(&mut KernelView<'a, T, B, W>) -> R,
    ) -> R {
        critical_section::with(|_cs| {
            // SAFETY: critical section ensures exclusive access to the table for duration of closure.
            unsafe { self.kernel_with_view_unchecked(f) }
        })
    }

    pub unsafe fn host_with_view_unchecked<R>(
        &self,
        f: impl FnOnce(&mut HostView<'_, T, B, W, P, S>) -> R,
    ) -> R {
        let table = unsafe { &mut *self.table.get() };
        let mut hv = HostView::new(table, &self.policy, &self.persist);
        f(&mut hv)
    }

    pub unsafe fn kernel_with_view_unchecked<R>(
        &self,
        f: impl FnOnce(&mut KernelView<'_, T, B, W>) -> R,
    ) -> R {
        let table = unsafe { &mut *self.table.get() };
        let mut kv = KernelView::new(table);
        f(&mut kv)
    }
}

#[cfg(feature = "staged")]
impl<const T: usize, const B: usize, const W: usize, P, S, St>
    ShadowStorageBase<T, B, W, P, S, WithStage<St>>
where
    P: AddressPolicy,
    S: PersistTrigger,
    St: crate::staged::internal::StagedImpl<T, B, W>,
{
    pub fn host_with_view_staged<R>(
        &self,
        f: impl FnOnce(&mut crate::view::HostViewStaged<'_, T, B, W, P, S, St>) -> R,
    ) -> R {
        critical_section::with(|_cs| unsafe {
            let table = &mut *self.table.get();
            let stage = &mut (*self.stage.get()).staged;

            let base = HostView::new(table, &self.policy, &self.persist);
            let mut hv = crate::view::HostViewStaged {
                base,
                staged: stage,
            };
            f(&mut hv)
        })
    }
}
