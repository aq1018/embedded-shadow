# embedded-shadow

[![Crates.io](https://img.shields.io/crates/v/embedded-shadow)](https://crates.io/crates/embedded-shadow)
[![docs.rs](https://docs.rs/embedded-shadow/badge.svg)](https://docs.rs/embedded-shadow)
[![CI](https://github.com/aq1018/embedded-shadow/actions/workflows/test.yml/badge.svg)](https://github.com/aq1018/embedded-shadow/actions/workflows/test.yml)

A `no_std`, no-alloc shadow register table for embedded systems with dirty tracking and transactional writes.

## Requirements

- **Rust 1.85+** (Edition 2024) - Uses const blocks for compile-time validation

## Features

- **Zero allocation** - All storage is statically allocated via const generics
- **Zero-copy access** - Direct slice access for reads and writes, no intermediate buffers
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
┌──────────────────┐         ┌────────────────────┐
│   Host (App)     │         │   Kernel (HW)      │
│                  │         │                    │
│  with_wo_slice() │────────▶│  iter_dirty()      │
│  (marks dirty)   │  dirty  │  (reads dirty)     │
│                  │  bits   │                    │
│                  │◀────────│  clear_all_dirty() │
│                  │  reset  │  with_rw_slice()   │
│                  │         │  (no dirty mark)   │
└──────────────────┘         └────────────────────┘
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

// Host side: write structured data using typed slice primitives
storage.host_shadow().with_view(|view| {
    // Write a config register: flags (u16) | timeout_ms (u16)
    view.with_wo_slice(0x100, 4, |mut slice| {
        slice.write_u16_le_at(0, 0x001F);  // flags
        slice.write_u16_le_at(2, 5000);    // timeout_ms
        WriteResult::Dirty(()) // mark dirty
    }).unwrap();
});

// Kernel side (ISR): sync dirty blocks to hardware
unsafe {
    storage.kernel_shadow().with_view_unchecked(|view| {
        view.iter_dirty(|addr, slice| {
            // Read typed values from the slice
            let flags = slice.read_u16_le_at(0);
            let timeout = slice.read_u16_le_at(2);
            // Write to hardware registers here...
            let _ = (flags, timeout);
            Ok(())
        }).unwrap();
        view.clear_all_dirty();
    });
}
```

## Choosing Block Size

Block size controls dirty tracking granularity. Consider:

- **Smaller blocks** (8-16 bytes): Finer tracking, more bitmap overhead, better for scattered small writes
- **Larger blocks** (64-256 bytes): Coarser tracking, less overhead, better for bulk writes

### Common Patterns

| Use Case | Typical Block Size | Rationale |
|----------|-------------------|-----------|
| SPI Flash | 256 bytes | Matches typical page size |
| I2C EEPROM | 32-64 bytes | Matches page write buffer |
| Motor controller registers | 16-32 bytes | Groups related parameters |
| Display frame buffer | 128-256 bytes | Balances granularity with DMA efficiency |
| General config | 64 bytes | Good default for mixed access patterns |

### Alignment Tips

- Align logical features to block boundaries when possible
- Power-of-2 sizes work well with typical memory layouts
- Match persist sector size if using flash persistence
- When iterating dirty blocks, you can match on block address to update only the affected feature

There's no universal "best" size—choose based on your access patterns and memory constraints.

## Address Space

Addresses use `u16`, limiting total storage to 64KB. This is sufficient for most embedded/MCU applications where shadow registers typically range from a few hundred bytes to a few kilobytes.

If your application requires larger address spaces, please [file an issue](https://github.com/aq1018/embedded-shadow/issues).

## Staging Buffer Sizing

When using transactional writes, size your `PatchStagingBuffer<DC, EC>`:

- **DC (Data Capacity)**: Total bytes of staged data. Sum the largest expected transaction.
- **EC (Entry Capacity)**: Maximum number of separate writes per transaction.

```rust
// Example: Up to 16 writes totaling 256 bytes
let staging = PatchStagingBuffer::<256, 16>::new();
```

If staging overflows, `alloc_staged()` returns `StageFull`. Start with generous estimates and tune down based on actual usage. For memory-constrained systems, consider:

- Smaller transactions with more frequent commits
- Combining adjacent writes into single larger writes
- Reducing EC if writes are always contiguous

## Documentation

- **[API Reference](https://docs.rs/embedded-shadow)** - Full API documentation
- **[Examples](examples/)** - Detailed usage patterns:
  - [`basic.rs`](examples/basic.rs) - Core concepts and dirty tracking
  - [`staging.rs`](examples/staging.rs) - Transactional writes with preview/commit/rollback
  - [`access_policy.rs`](examples/access_policy.rs) - Memory protection and access control
  - [`persist.rs`](examples/persist.rs) - Flash persistence patterns
  - [`complex.rs`](examples/complex.rs) - Real-world motor controller simulation

## Thread Safety

`ShadowStorage` uses interior mutability via `UnsafeCell` and is **not `Sync`** by default. Safe concurrent access is provided through:

- **`with_view()`** - Wraps access in a `critical_section`, safe for ISR/main coordination
- **`with_view_unchecked()`** - Bypasses critical section for performance when caller guarantees exclusive access

For multi-core scenarios, wrap storage access in platform-specific synchronization primitives.

## Critical Section

This crate requires a `critical-section` implementation for your platform. Most embedded HALs provide this. For testing, add:

```toml
[dev-dependencies]
critical-section = { version = "1.2", features = ["std"] }
```

## Safety

This crate uses `#![deny(unsafe_code)]` at the crate root, with `unsafe` allowed only in `storage.rs` for interior mutability.

### Interior Mutability

`ShadowStorage` uses `UnsafeCell` to allow mutation through shared references. Safety is ensured by:

1. **Critical sections**: `with_view()` wraps all access in `critical_section::with()`
2. **Single-threaded guarantee**: `with_view_unchecked()` requires caller to ensure exclusive access (typically in ISR context or before interrupts are enabled)

### Invariants

- Only one view (Host or Kernel) should be active at a time
- `with_view_unchecked()` is safe when:
  - Interrupts are disabled
  - Called from an ISR that cannot be preempted
  - Running single-threaded during initialization

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
