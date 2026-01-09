#![deny(unsafe_code)]
#![no_std]

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

pub use error::ShadowError;
pub use persist::{NoPersist, PersistTrigger};
pub use policy::{AccessPolicy, AllowAllPolicy};
pub use storage::ShadowStorage;
pub use view::HostView;

pub mod prelude {
    pub use crate::{
        AccessPolicy, AllowAllPolicy, HostView, NoPersist, PersistTrigger, ShadowError,
        ShadowStorage,
    };
}
