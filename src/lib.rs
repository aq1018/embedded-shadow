//! A `no_std`, no-alloc shadow register table for embedded systems.
//!
//! This crate provides efficient shadow register management with dirty tracking,
//! suitable for memory-mapped I/O, peripheral register caching, and state synchronization
//! between application and hardware layers.
//!
//! # Features
//!
//! - **Zero heap allocation** - All storage statically allocated
//! - **Block-based dirty tracking** - Efficiently track modifications
//! - **Dual access patterns** - Host (application) and Kernel (hardware) views
//! - **Flexible policies** - Customizable access control and persistence
//! - **Transactional writes** - Optional staging with commit/rollback
//!
//! # Architecture
//!
//! The shadow registry uses a **one-way dirty tracking model**:
//!
//! ```text
//! ┌──────────────────┐         ┌──────────────────────────┐
//! │   Host (App)     │         │   Kernel (HW)            │
//! │                  │         │                          │
//! │  write_range()   │────────▶│  for_each_dirty_block()  │
//! │  (marks dirty)   │  dirty  │  (reads dirty)           │
//! │                  │  bits   │                          │
//! │                  │◀────────│  clear_dirty()           │
//! │                  │  reset  │  write_range()           │
//! │                  │         │  (no dirty mark)         │
//! └──────────────────┘         └──────────────────────────┘
//! ```
//!
//! - **Host writes** mark blocks as dirty and may trigger persistence
//! - **Kernel reads** dirty state to sync changes to hardware
//! - **Kernel writes** update the shadow (e.g., after reading from hardware) without marking dirty
//! - **Kernel clears** dirty bits after syncing
//!
//! This design enables efficient one-way synchronization from application to hardware.
//!
//! # Example
//!
//! ```rust,no_run
//! use embedded_shadow::prelude::*;
//!
//! let storage = ShadowStorageBuilder::new()
//!     .total_size::<1024>()
//!     .block_size::<64>()
//!     .block_count::<16>()
//!     .default_access()
//!     .no_persist()
//!     .build();
//! ```

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
pub(crate) mod table;
pub mod types;
pub mod view;

pub use builder::ShadowStorageBuilder;
pub use error::ShadowError;
pub use persist::{NoPersist, PersistTrigger};
pub use policy::{AccessPolicy, AllowAllPolicy, NoPersistPolicy, PersistPolicy};
pub use shadow::{HostShadow, KernelShadow};
pub use staged::PatchStagingBuffer;
pub use storage::ShadowStorage;
pub use types::StagingBuffer;
pub use view::{HostView, HostViewStaged, KernelView};

pub mod prelude {
    pub use crate::{
        builder::ShadowStorageBuilder,
        error::ShadowError,
        persist::{NoPersist, PersistTrigger},
        policy::{AccessPolicy, AllowAllPolicy, NoPersistPolicy, PersistPolicy},
        shadow::{HostShadow, KernelShadow},
        staged::PatchStagingBuffer,
        storage::ShadowStorage,
        types::StagingBuffer,
        view::{HostView, HostViewStaged, KernelView},
    };
}
