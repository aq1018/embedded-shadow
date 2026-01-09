# embedded-shadow

[![Crates.io](https://img.shields.io/crates/v/embedded-shadow)](https://crates.io/crates/embedded-shadow)
[![docs.rs](https://docs.rs/embedded-shadow/badge.svg)](https://docs.rs/embedded-shadow)
[![CI](https://github.com/aq1018/embedded-shadow/actions/workflows/test.yml/badge.svg)](https://github.com/aq1018/embedded-shadow/actions/workflows/test.yml)
[![License](https://img.shields.io/crates/l/embedded-shadow)](LICENSE-MIT)
[![Minimum rustc version](https://img.shields.io/badge/rustc-1.85+-lightgray.svg)](https://github.com/rust-lang/rust)

A `no_std`, no-alloc shadow register table for embedded systems with dirty tracking and transactional writes.

## Features

- **Zero allocation** - All storage is statically allocated via const generics
- **Dirty tracking** - Efficiently track which blocks have been modified
- **Dual views** - Separate Host (application) and Kernel (hardware) access patterns
- **Access policies** - Control read/write permissions for different memory regions
- **Persistence policies** - Define what and when data should be persisted
- **Staging support** - Preview and commit/rollback transactional writes
- **Critical-section support** - Thread-safe access when needed

## Quick Start

```rust
use embedded_shadow::prelude::*;

// Create a 1KB shadow register table
let storage = ShadowStorageBuilder::new()
    .total_size::<1024>()
    .block_size::<64>()
    .block_count::<16>()
    .default_access()
    .no_persist()
    .build();

// Host writes data (marks dirty)
let host = storage.host_shadow();
host.with_view(|view| {
    view.write_range(0x100, &[0x01, 0x02, 0x03, 0x04])?;
    Ok(())
});

// Kernel syncs dirty blocks to hardware
let kernel = storage.kernel_shadow();
kernel.with_view(|view| {
    view.for_each_dirty_block(|addr, data| {
        // Write to hardware registers
        hardware_write(addr, data);
        Ok(())
    })?;
    view.clear_dirty();
    Ok(())
});
```

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
