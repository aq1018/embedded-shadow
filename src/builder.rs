use core::marker::PhantomData;

use bitmaps::{Bits, BitsImpl};

use crate::{
    persist::{NoPersist, PersistTrigger},
    policy::{AccessPolicy, AllowAllPolicy, NoPersistPolicy, PersistPolicy},
    storage::ShadowStorage,
};

// Builder states
pub struct NeedTotalSize;
pub struct NeedBlockSize;
pub struct NeedBlockCount;
pub struct NeedAccessPolicy;
pub struct NeedPersistPolicy;
pub struct NeedPersistTrigger;
pub struct Ready;

#[derive(Default)]
pub struct ShadowStorageBuilder<
    const TS: usize,
    const BS: usize,
    const BC: usize,
    AP,
    PP,
    PT,
    PK,
    State,
> {
    access_policy: Option<AP>,
    persist_policy: Option<PP>,
    persist_trigger: Option<PT>,
    _phantom: PhantomData<(PK, State)>,
}

// Start the builder
impl ShadowStorageBuilder<0, 0, 0, (), (), (), (), NeedTotalSize> {
    pub fn new() -> Self {
        ShadowStorageBuilder {
            access_policy: None,
            persist_policy: None,
            persist_trigger: None,
            _phantom: PhantomData,
        }
    }
}

// Set total size
impl ShadowStorageBuilder<0, 0, 0, (), (), (), (), NeedTotalSize> {
    pub fn total_size<const TS: usize>(
        self,
    ) -> ShadowStorageBuilder<TS, 0, 0, (), (), (), (), NeedBlockSize> {
        ShadowStorageBuilder {
            access_policy: None,
            persist_policy: None,
            persist_trigger: None,
            _phantom: PhantomData,
        }
    }
}

// Set block size
impl<const TS: usize> ShadowStorageBuilder<TS, 0, 0, (), (), (), (), NeedBlockSize> {
    pub fn block_size<const BS: usize>(
        self,
    ) -> ShadowStorageBuilder<TS, BS, 0, (), (), (), (), NeedBlockCount> {
        ShadowStorageBuilder {
            access_policy: None,
            persist_policy: None,
            persist_trigger: None,
            _phantom: PhantomData,
        }
    }
}

// Set block count
impl<const TS: usize, const BS: usize>
    ShadowStorageBuilder<TS, BS, 0, (), (), (), (), NeedBlockCount>
{
    /// Set the number of blocks.
    ///
    /// # Panics
    /// Panics at runtime if TS != BS * BC.
    /// For a 1024-byte storage with 64-byte blocks, use BC = 16.
    pub fn block_count<const BC: usize>(
        self,
    ) -> ShadowStorageBuilder<TS, BS, BC, (), (), (), (), NeedAccessPolicy> {
        // Early validation - fail fast with clear error message
        assert_eq!(
            TS,
            BS * BC,
            "Total size {} does not match block_size {} * block_count {} = {}",
            TS,
            BS,
            BC,
            BS * BC
        );

        ShadowStorageBuilder {
            access_policy: None,
            persist_policy: None,
            persist_trigger: None,
            _phantom: PhantomData,
        }
    }
}

// Set access policy
impl<const TS: usize, const BS: usize, const BC: usize>
    ShadowStorageBuilder<TS, BS, BC, (), (), (), (), NeedAccessPolicy>
{
    pub fn access_policy<AP: AccessPolicy>(
        self,
        policy: AP,
    ) -> ShadowStorageBuilder<TS, BS, BC, AP, (), (), (), NeedPersistPolicy> {
        ShadowStorageBuilder {
            access_policy: Some(policy),
            persist_policy: None,
            persist_trigger: None,
            _phantom: PhantomData,
        }
    }

    /// Use the default allow-all access policy
    pub fn default_access(
        self,
    ) -> ShadowStorageBuilder<TS, BS, BC, AllowAllPolicy, (), (), (), NeedPersistPolicy> {
        self.access_policy(AllowAllPolicy::default())
    }
}

// Set persist policy
impl<const TS: usize, const BS: usize, const BC: usize, AP>
    ShadowStorageBuilder<TS, BS, BC, AP, (), (), (), NeedPersistPolicy>
where
    AP: AccessPolicy,
{
    /// Set a custom persist policy with a specific key type
    pub fn persist_policy<PP, PK>(
        self,
        policy: PP,
    ) -> ShadowStorageBuilder<TS, BS, BC, AP, PP, (), PK, NeedPersistTrigger>
    where
        PP: PersistPolicy<PK>,
    {
        ShadowStorageBuilder {
            access_policy: self.access_policy,
            persist_policy: Some(policy),
            persist_trigger: None,
            _phantom: PhantomData,
        }
    }

    /// Use no persistence (no persist policy or trigger)
    pub fn no_persist(
        self,
    ) -> ShadowStorageBuilder<TS, BS, BC, AP, NoPersistPolicy, NoPersist, (), Ready> {
        ShadowStorageBuilder {
            access_policy: self.access_policy,
            persist_policy: Some(NoPersistPolicy::default()),
            persist_trigger: Some(NoPersist),
            _phantom: PhantomData,
        }
    }
}

// Set persist trigger
impl<const TS: usize, const BS: usize, const BC: usize, AP, PP, PK>
    ShadowStorageBuilder<TS, BS, BC, AP, PP, (), PK, NeedPersistTrigger>
where
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
{
    /// Set the persist trigger that handles the persistence keys
    pub fn persist_trigger<PT>(
        self,
        trigger: PT,
    ) -> ShadowStorageBuilder<TS, BS, BC, AP, PP, PT, PK, Ready>
    where
        PT: PersistTrigger<PK>,
    {
        ShadowStorageBuilder {
            access_policy: self.access_policy,
            persist_policy: self.persist_policy,
            persist_trigger: Some(trigger),
            _phantom: PhantomData,
        }
    }
}

// Build the final storage
impl<const TS: usize, const BS: usize, const BC: usize, AP, PP, PT, PK>
    ShadowStorageBuilder<TS, BS, BC, AP, PP, PT, PK, Ready>
where
    AP: AccessPolicy,
    PP: PersistPolicy<PK>,
    PT: PersistTrigger<PK>,
    BitsImpl<BC>: Bits,
{
    /// Build the final ShadowStorage instance
    ///
    /// # Panics
    /// Panics if TS != BS * BC (validated in ShadowTable::new)
    pub fn build(self) -> ShadowStorage<TS, BS, BC, AP, PP, PT, PK> {
        ShadowStorage::new(
            self.access_policy.unwrap(),
            self.persist_policy.unwrap(),
            self.persist_trigger.unwrap(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_builder() {
        let _storage = ShadowStorageBuilder::new()
            .total_size::<1024>()
            .block_size::<64>()
            .block_count::<16>() // 64 * 16 = 1024
            .default_access()
            .no_persist()
            .build();
    }

    #[test]
    fn test_builder_with_custom_policies() {
        struct TestAccessPolicy;
        impl AccessPolicy for TestAccessPolicy {
            fn can_read(&self, _addr: u16, _len: usize) -> bool {
                true
            }
            fn can_write(&self, _addr: u16, _len: usize) -> bool {
                true
            }
        }

        struct TestPersistPolicy;
        impl PersistPolicy<u32> for TestPersistPolicy {
            fn push_persist_keys_for_range<F>(&self, _addr: u16, _len: usize, _push_key: F) -> bool
            where
                F: FnMut(u32),
            {
                false
            }
        }

        struct TestPersistTrigger;
        impl PersistTrigger<u32> for TestPersistTrigger {
            fn push_key(&mut self, _key: u32) {}
            fn request_persist(&mut self) {}
        }

        let _storage = ShadowStorageBuilder::new()
            .total_size::<2048>()
            .block_size::<128>()
            .block_count::<16>() // 128 * 16 = 2048
            .access_policy(TestAccessPolicy)
            .persist_policy(TestPersistPolicy)
            .persist_trigger(TestPersistTrigger)
            .build();
    }

    #[test]
    #[should_panic(expected = "Total size 1024 does not match block_size 64 * block_count 15")]
    fn test_builder_panics_on_mismatch() {
        let _storage = ShadowStorageBuilder::new()
            .total_size::<1024>()
            .block_size::<64>()
            .block_count::<15>() // 64 * 15 = 960, not 1024!
            .default_access()
            .no_persist()
            .build();
    }
}
