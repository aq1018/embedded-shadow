#![deny(unsafe_code)]
#![no_std]

pub mod builder;
pub mod error;
pub mod helpers;
pub mod persist;
pub mod policy;
pub mod shadow;
pub mod staged;
pub mod storage;
pub mod table;
pub mod types;
pub mod view;

pub use builder::ShadowStorageBuilder;
pub use error::ShadowError;
pub use persist::{NoPersist, PersistTrigger};
pub use policy::{AccessPolicy, AllowAllPolicy, NoPersistPolicy, PersistPolicy};
pub use staged::PatchStagingBuffer;
pub use storage::ShadowStorage;
pub use types::StagingBuffer;
pub use view::HostView;

pub mod prelude {
    pub use crate::{
        builder::ShadowStorageBuilder,
        error::ShadowError,
        persist::{NoPersist, PersistTrigger},
        policy::{AccessPolicy, AllowAllPolicy, NoPersistPolicy, PersistPolicy},
        staged::PatchStagingBuffer,
        storage::ShadowStorage,
        types::StagingBuffer,
        view::HostView,
    };
}
