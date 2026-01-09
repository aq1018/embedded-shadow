# embedded-shadow

[![Crates.io](https://img.shields.io/crates/v/embedded-shadow)](https://crates.io/crates/embedded-shadow)
[![docs.rs](https://docs.rs/embedded-shadow/badge.svg)](https://docs.rs/embedded-shadow)
[![CI](https://github.com/aq1018/embedded-shadow/actions/workflows/test.yml/badge.svg)](https://github.com/aq1018/embedded-shadow/actions/workflows/test.yml)

A `no_std`, no-alloc shadow register table for embedded systems with dirty tracking and transactional writes.

## Features

- **Zero allocation** - All storage is statically allocated via const generics
- **Dirty tracking** - Efficiently track which blocks have been modified
- **Dual views** - Separate Host (application) and Kernel (hardware) access patterns
- **Access policies** - Control read/write permissions for different memory regions
- **Persistence policies** - Define what and when data should be persisted
- **Staging support** - Preview and commit/rollback transactional writes
- **Critical-section support** - Thread-safe access when needed

## Use Cases

- **Cache hardware registers in RAM** - Avoid expensive read-modify-write cycles to slow peripherals (SPI/I2C devices, external memory-mapped chips)
- **Track what changed** - Dirty tracking lets you sync only modified data to hardware, reducing bus traffic
- **Decouple app from hardware timing** - Application writes to shadow anytime; hardware driver syncs during appropriate windows (ISR, DMA idle)
- **Preview before commit** - Staging support lets you build up changes and preview the result before committing (useful for configuration UIs, parameter tuning)
- **Persist selectively** - Policy-based persistence triggers let you save only specific regions to flash/EEPROM

**Typical applications:** motor controller parameter tables, sensor configuration registers, display controller buffers, audio codec settings, or any peripheral with a register map you want to cache and batch-update.

## Architecture

The shadow registry uses a **one-way dirty tracking model**:

```text
┌──────────────────┐         ┌──────────────────────────┐
│   Host (App)     │         │   Kernel (HW)            │
│                  │         │                          │
│  write_range()   │────────▶│  for_each_dirty_block()  │
│  (marks dirty)   │  dirty  │  (reads dirty)           │
│                  │  bits   │                          │
│                  │◀────────│  clear_dirty()           │
│                  │  reset  │  write_range()           │
│                  │         │  (no dirty mark)         │
└──────────────────┘         └──────────────────────────┘
```

- **Host writes** mark blocks as dirty and may trigger persistence
- **Kernel reads** dirty state to sync changes to hardware
- **Kernel writes** update the shadow (e.g., after reading from hardware) without marking dirty
- **Kernel clears** dirty bits after syncing

This design enables efficient one-way synchronization from application to hardware.

## Quick Start

```rust
use embedded_shadow::prelude::*;

// Create storage: 1KB total, 64-byte blocks, 16 blocks
let storage = ShadowStorageBuilder::new()
    .total_size::<1024>()
    .block_size::<64>()
    .block_count::<16>()
    .default_access()
    .no_persist()
    .build();

// Load factory defaults at boot (doesn't mark dirty)
storage.load_defaults(|write| {
    write(0x000, &[0x01, 0x02, 0x03, 0x04])?;
    write(0x100, &[0xAA, 0xBB, 0xCC, 0xDD])?;
    Ok(())
}).unwrap();

// host_shadow() and kernel_shadow() return short-lived references,
// typically constructed each iteration and passed via context, e.g.:
//   fn update(ctx: &mut HostContext) { ctx.shadow.with_view(|view| { ... }); }
//   fn run_once(ctx: &mut KernelContext) { ctx.shadow.with_view_unchecked(|view| { ... }); }

// Host side (main loop): use with_view for critical section safety
storage.host_shadow().with_view(|view| {
    // Handle errors explicitly with match
    match view.write_range(0x100, &[0xDE, 0xAD, 0xBE, 0xEF]) {
        Ok(()) => {}
        Err(ShadowError::Denied) => { /* handle access denied */ }
        Err(ShadowError::OutOfBounds) => { /* handle invalid range */ }
        Err(e) => panic!("unexpected error: {:?}", e),
    }

    let mut buf = [0u8; 4];
    view.read_range(0x100, &mut buf).unwrap();
});

// Kernel side (ISR): use with_view_unchecked since ISR already has exclusive access
unsafe {
    storage.kernel_shadow().with_view_unchecked(|view| {
        view.for_each_dirty_block(|addr, data| {
            // Write to hardware registers here
            Ok(())
        }).unwrap();
        view.clear_dirty();
    });
}
```

## Choosing Block Size

Block size controls dirty tracking granularity. Some considerations:

- **Smaller blocks** (e.g., 8-16 bytes): Finer tracking, more dirty bits, better for scattered small writes
- **Larger blocks** (e.g., 64-256 bytes): Coarser tracking, fewer dirty bits, better for sequential or bulk writes

Common patterns:
- Match your hardware's natural transfer size (SPI page, I2C buffer)
- Match your persist sector size if using flash persistence
- Align features to blocks so each block represents one logical feature—when iterating dirty blocks, you can match on block address to update only the affected feature
- Power of 2 sizes work well with typical memory layouts

There's no universal "best" size—choose based on your access patterns and memory constraints.

## Examples

See the [examples](examples/) directory for detailed usage:

- [`basic.rs`](examples/basic.rs) - Core concepts and dirty tracking
- [`staging.rs`](examples/staging.rs) - Transactional writes with preview/commit/rollback
- [`access_policy.rs`](examples/access_policy.rs) - Memory protection and access control
- [`persist.rs`](examples/persist.rs) - Flash persistence patterns
- [`complex.rs`](examples/complex.rs) - Real-world motor controller simulation

## Critical Section

This crate requires a `critical-section` implementation for your platform. Most embedded HALs provide this. For testing, add:

```toml
[dev-dependencies]
critical-section = { version = "1.2", features = ["std"] }
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
