//! Test support utilities - only compiled in test builds.

use crate::shadow::{
    ShadowError, WriteResult,
    persist::{NoPersist, PersistTrigger},
    policy::{AccessPolicy, AllowAllPolicy, NoPersistPolicy, PersistPolicy},
    staged::PatchStagingBuffer,
    storage::ShadowStorage,
    table::ShadowTable,
    types::StagingBuffer,
    view::{HostView, HostViewStaged},
};

/// Standard test configuration: 64 bytes, 16-byte blocks, 4 blocks
pub type TestTable = ShadowTable<64, 16, 4>;
pub type TestStorage = ShadowStorage<64, 16, 4, AllowAllPolicy, NoPersistPolicy, NoPersist, ()>;
pub type TestStage = PatchStagingBuffer<64, 8>;

/// Policy that denies all access
pub struct DenyAllPolicy;

impl AccessPolicy for DenyAllPolicy {
    fn can_read(&self, _addr: u16, _len: usize) -> bool {
        false
    }
    fn can_write(&self, _addr: u16, _len: usize) -> bool {
        false
    }
}

/// Policy that allows reads everywhere but only writes at addr >= 32
pub struct ReadOnlyBelow32;

impl AccessPolicy for ReadOnlyBelow32 {
    fn can_read(&self, _addr: u16, _len: usize) -> bool {
        true
    }
    fn can_write(&self, addr: u16, len: usize) -> bool {
        addr >= 32 && (addr as usize + len) <= 64
    }
}

/// Helper to create a default test storage
pub fn test_storage() -> TestStorage {
    ShadowStorage::new(
        AllowAllPolicy::default(),
        NoPersistPolicy::default(),
        NoPersist,
    )
}

/// Helper to stage writes in tests - always commits the write.
pub fn stage_write(stage: &mut TestStage, addr: u16, data: &[u8]) -> Result<(), ShadowError> {
    stage.alloc_staged(addr, data.len(), |buf| {
        buf.copy_from_slice(data);
        WriteResult::Dirty(())
    })?;
    Ok(())
}

/// Asserts that bytes at the given address match expected data.
pub fn assert_table_bytes(table: &TestTable, addr: u16, expected: &[u8]) {
    table
        .with_bytes(addr, expected.len(), |data| {
            assert_eq!(data, expected);
            Ok(())
        })
        .unwrap();
}

/// Fixture for HostView tests with default policies.
pub struct TestHostViewFixture {
    pub table: TestTable,
    pub policy: AllowAllPolicy,
    pub persist_policy: NoPersistPolicy,
    pub trigger: NoPersist,
}

impl TestHostViewFixture {
    pub fn new() -> Self {
        Self {
            table: TestTable::new(),
            policy: AllowAllPolicy::default(),
            persist_policy: NoPersistPolicy::default(),
            trigger: NoPersist,
        }
    }

    pub fn view(
        &mut self,
    ) -> HostView<'_, 64, 16, 4, AllowAllPolicy, NoPersistPolicy, NoPersist, ()> {
        HostView::new(
            &mut self.table,
            &self.policy,
            &self.persist_policy,
            &mut self.trigger,
        )
    }
}

impl Default for TestHostViewFixture {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixture for HostViewStaged tests with default policies.
pub struct TestHostViewStagedFixture {
    pub table: TestTable,
    pub policy: AllowAllPolicy,
    pub persist_policy: NoPersistPolicy,
    pub trigger: NoPersist,
    pub stage: TestStage,
}

impl TestHostViewStagedFixture {
    pub fn new() -> Self {
        Self {
            table: TestTable::new(),
            policy: AllowAllPolicy::default(),
            persist_policy: NoPersistPolicy::default(),
            trigger: NoPersist,
            stage: TestStage::new(),
        }
    }

    pub fn view(
        &mut self,
    ) -> HostViewStaged<'_, 64, 16, 4, AllowAllPolicy, NoPersistPolicy, NoPersist, (), TestStage>
    {
        let base = HostView::new(
            &mut self.table,
            &self.policy,
            &self.persist_policy,
            &mut self.trigger,
        );
        HostViewStaged::new(base, &mut self.stage)
    }
}

impl Default for TestHostViewStagedFixture {
    fn default() -> Self {
        Self::new()
    }
}

/// Asserts that the result is a Denied error.
pub fn assert_denied<T: core::fmt::Debug>(result: Result<T, ShadowError>) {
    assert_eq!(result.unwrap_err(), ShadowError::Denied);
}

/// A persist trigger that tracks whether persist was requested.
#[derive(Default)]
pub struct TrackingPersistTrigger {
    pub persist_requested: bool,
}

impl PersistTrigger<()> for TrackingPersistTrigger {
    fn push_key(&mut self, _key: ()) {}

    fn request_persist(&mut self) {
        self.persist_requested = true;
    }
}

/// A persist policy that always triggers persistence.
#[derive(Default)]
pub struct AlwaysPersistPolicy;

impl PersistPolicy<()> for AlwaysPersistPolicy {
    fn push_persist_keys_for_range<F>(&self, _addr: u16, _len: usize, _push: F) -> bool
    where
        F: FnMut(()),
    {
        true
    }
}
