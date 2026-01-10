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

#[cfg(test)]
mod test_support;

pub use builder::ShadowStorageBuilder;
pub use error::ShadowError;
pub use handle::{HostShadow, KernelShadow};
pub use persist::{NoPersist, PersistTrigger};
pub use policy::{AccessPolicy, AllowAllPolicy, NoPersistPolicy, PersistPolicy};
pub use staged::PatchStagingBuffer;
pub use storage::{ShadowStorage, WriteFn};
pub use types::StagingBuffer;
pub use view::{HostView, HostViewStaged, KernelView};

pub mod prelude {
    pub use super::{
        AccessPolicy, AllowAllPolicy, HostShadow, HostView, HostViewStaged, KernelShadow,
        KernelView, NoPersist, NoPersistPolicy, PatchStagingBuffer, PersistPolicy, PersistTrigger,
        ShadowError, ShadowStorage, ShadowStorageBuilder, StagingBuffer,
    };
}
