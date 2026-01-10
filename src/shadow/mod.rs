pub mod builder;
pub mod error;
pub mod handle;
pub mod helpers;
pub mod persist;
pub mod policy;
pub mod staged;
pub mod storage;
pub(crate) mod table;
pub mod types;
pub mod view;

pub use builder::ShadowStorageBuilder;
pub use error::ShadowError;
pub use handle::{HostShadow, KernelShadow};
pub use persist::{NoPersist, PersistTrigger};
pub use policy::{AccessPolicy, AllowAllPolicy, NoPersistPolicy, PersistPolicy};
pub use staged::PatchStagingBuffer;
pub use storage::{ShadowStorage, WriteFn};
pub use types::StagingBuffer;
pub use view::{HostView, HostViewStaged, KernelView};

#[cfg(test)]
mod test_support;

pub mod prelude {
    pub use crate::{
        builder::ShadowStorageBuilder,
        error::ShadowError,
        handle::{HostShadow, KernelShadow},
        persist::{NoPersist, PersistTrigger},
        policy::{AccessPolicy, AllowAllPolicy, NoPersistPolicy, PersistPolicy},
        staged::PatchStagingBuffer,
        storage::ShadowStorage,
        types::StagingBuffer,
        view::{HostView, HostViewStaged, KernelView},
    };
}
