#![deny(unsafe_code)]
#![no_std]

#[cfg(all(feature = "staged-mirror", feature = "staged-patch"))]
compile_error!("Enable only one: staged-mirror OR staged-patch");

pub mod error;
pub mod ops;
pub mod persist;
pub mod policy;
pub mod staged;
pub mod storage;
pub mod table;
pub mod view;

pub use error::ShadowError;
pub use persist::{NoPersist, PersistTrigger};
pub use policy::{AddressPolicy, AllowAllPolicy};
pub use storage::ShadowStorage;
pub use view::{HostView, KernelView};

pub mod prelude {
    pub use crate::{
        AddressPolicy, AllowAllPolicy, HostView, KernelView, NoPersist, PersistTrigger,
        ShadowError, ShadowStorage,
    };
}
