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
//! │  with_wo_slice() │────────▶│  iter_dirty()            │
//! │  (marks dirty)   │  dirty  │  (reads dirty)           │
//! │                  │  bits   │                          │
//! │                  │◀────────│  clear_dirty()           │
//! │                  │  reset  │  with_rw_slice()         │
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
//! // Create storage: 1KB total, 64-byte blocks, 16 blocks
//! let storage = ShadowStorageBuilder::new()
//!     .total_size::<1024>()
//!     .block_size::<64>()
//!     .block_count::<16>()
//!     .default_access()
//!     .no_persist()
//!     .build();
//!
//! // Host side: write structured data using typed slice primitives
//! storage.host_shadow().with_view(|view| {
//!     // Write a config register: flags (u16) | timeout_ms (u16)
//!     view.with_wo_slice(0x100, 4, |mut slice| {
//!         slice.write_u16_le_at(0, 0x001F);  // flags
//!         slice.write_u16_le_at(2, 5000);    // timeout_ms
//!         (true, ()) // mark dirty
//!     }).unwrap();
//! });
//!
//! // Kernel side (ISR): sync dirty blocks to hardware
//! unsafe {
//!     storage.kernel_shadow().with_view_unchecked(|view| {
//!         view.iter_dirty(|addr, slice| {
//!             // Read typed values from the slice
//!             let flags = slice.read_u16_le_at(0);
//!             let timeout = slice.read_u16_le_at(2);
//!             // Write to hardware registers here...
//!             let _ = (flags, timeout);
//!             Ok(())
//!         }).unwrap();
//!         view.clear_dirty();
//!     });
//! }
//! ```

#![deny(unsafe_code)]
#![no_std]

pub mod shadow;

pub mod prelude {
    pub use crate::shadow::prelude::*;
}
