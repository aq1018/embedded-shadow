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
//! // Create storage: 1KB total, 64-byte blocks, 16 blocks
//! let storage = ShadowStorageBuilder::new()
//!     .total_size::<1024>()
//!     .block_size::<64>()
//!     .block_count::<16>()
//!     .default_access()
//!     .no_persist()
//!     .build();
//!
//! // Load factory defaults at boot (doesn't mark dirty)
//! storage.load_defaults(|write| {
//!     write(0x000, &[0x01, 0x02, 0x03, 0x04])?;
//!     write(0x100, &[0xAA, 0xBB, 0xCC, 0xDD])?;
//!     Ok(())
//! }).unwrap();
//!
//! // host_shadow() and kernel_shadow() return short-lived references,
//! // typically constructed each iteration and passed via context, e.g.:
//! //   fn update(ctx: &mut HostContext) { ctx.shadow.with_view(|view| { ... }); }
//! //   fn run_once(ctx: &mut KernelContext) { ctx.shadow.with_view_unchecked(|view| { ... }); }
//!
//! // Host side (main loop): use with_view for critical section safety
//! storage.host_shadow().with_view(|view| {
//!     // Handle errors explicitly with match
//!     match view.write_range(0x100, &[0xDE, 0xAD, 0xBE, 0xEF]) {
//!         Ok(()) => {}
//!         Err(ShadowError::Denied) => { /* handle access denied */ }
//!         Err(ShadowError::OutOfBounds) => { /* handle invalid range */ }
//!         Err(e) => panic!("unexpected error: {:?}", e),
//!     }
//!
//!     let mut buf = [0u8; 4];
//!     view.read_range(0x100, &mut buf).unwrap();
//! });
//!
//! // Kernel side (ISR): use with_view_unchecked since ISR already has exclusive access
//! unsafe {
//!     storage.kernel_shadow().with_view_unchecked(|view| {
//!         view.for_each_dirty_block(|addr, data| {
//!             // Write to hardware registers here
//!             Ok(())
//!         }).unwrap();
//!         view.clear_dirty();
//!     });
//! }
//! ```

#![deny(unsafe_code)]
#![no_std]

pub mod shadow;

// Key types re-exported at crate root for convenience
pub use shadow::{AccessPolicy, PersistPolicy, ShadowError, ShadowStorage, ShadowStorageBuilder};

pub mod prelude {
    pub use crate::shadow::prelude::*;
}
