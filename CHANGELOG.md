# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-01-10

### Added

- Zero-copy slice wrappers: `ROSlice`, `WOSlice`, `RWSlice` with typed primitive accessors
  - Read methods: `read_u8_at`, `read_u16_le_at`, `read_u16_be_at`, `read_u32_le_at`, `read_u32_be_at` (and signed variants)
  - Write methods: `write_u8_at`, `write_u16_le_at`, `write_u16_be_at`, `write_u32_le_at`, `write_u32_be_at` (and signed variants)
  - Copy methods: `copy_to_slice`, `copy_to_slice_at`, `copy_from_slice`, `copy_from_slice_at`
  - Fill methods: `fill`, `fill_at`
- Fallible `try_*` variants for all slice primitives returning `Option<T>` instead of panicking
  - `try_read_u8_at`, `try_read_u16_le_at`, `try_write_u32_be_at`, etc.
  - `try_copy_to_slice_at`, `try_copy_from_slice_at`, `try_fill_at`
- `WriteResult<R>` enum for explicit dirty marking intent
  - `WriteResult::Dirty(R)` - marks range as modified
  - `WriteResult::Clean(R)` - skips dirty marking
  - Methods: `is_dirty()`, `into_inner()`
- Compile-time builder validation using const blocks (requires Rust 1.85+)
  - Fails at compile time if `TS != BS * BC` instead of runtime panic
- `Display` impl for `ShadowError`
- `Debug` impl for `ShadowStorageBuilder`, `HostView`, `KernelView`, and slice types
- README documentation:
  - Requirements section with MSRV (Rust 1.85+)
  - Address Space section documenting `u16` limit (64KB)
  - Thread Safety section explaining `UnsafeCell` and critical section usage
  - Safety section with interior mutability invariants
  - Staging Buffer Sizing guidance
  - Expanded Choosing Block Size with common patterns table

### Changed

- **Breaking:** Complete API redesign for zero-copy access
  - `HostView`:
    - `read_range(addr, &mut buf)` → `with_ro_slice(addr, len, |slice| ...)` returning `Result<R, ShadowError>`
    - `write_range(addr, &data)` → `with_wo_slice(addr, len, |slice| WriteResult::Dirty(...))` returning `Result<WriteResult<R>, ShadowError>`
    - New `with_rw_slice()` for read-modify-write operations
  - `KernelView`:
    - `read_range(addr, &mut buf)` → `with_ro_slice(addr, len, |slice| ...)`
    - `write_range(addr, &data)` → `with_rw_slice(addr, len, |slice| ...)`
    - `for_each_dirty_block(|addr, &[u8]|)` → `iter_dirty(|addr, ROSlice|)`
- **Breaking:** `StagingBuffer` trait redesigned for zero-copy
  - `write_staged(addr, &data)` → `alloc_staged(addr, len, |&mut [u8]| WriteResult::Dirty(()))`
  - `for_each_staged()` → `iter_staged()`
  - Removed `apply_overlay()`
- **Breaking:** `HostViewStaged`:
  - `stage_write(addr, &data)` → `alloc_staged(addr, len, |slice| WriteResult::Dirty(()))`
  - `action()` → `commit_staged()` (was deprecated in 0.1.2)
- **Breaking:** Storage initialization API
  - `load_defaults(|write| write(addr, &data))` → `with_defaults(addr, len, |WOSlice| ...)`
  - `load_defaults_unchecked()` → `with_defaults_unchecked()`
- **Breaking:** Dirty clearing methods renamed
  - `KernelView::clear_dirty()` (clear all) → `clear_all_dirty()`
  - New `clear_dirty(addr, len)` for range-based clearing
- Module reorganization: internals moved to `shadow` module, public API re-exported via prelude

### Removed

- `HostView::read_range()`, `HostView::write_range()`
- `KernelView::read_range()`, `KernelView::write_range()`, `KernelView::for_each_dirty_block()`
- `StagingBuffer::apply_overlay()`, `StagingBuffer::write_staged()`
- `HostViewStaged::read_range()`, `HostViewStaged::stage_write()`, `HostViewStaged::action()`

## [0.1.2] - 2026-01-09

### Added

- `load_defaults()` and `load_defaults_unchecked()` for initializing shadow table with factory/EEPROM data without marking dirty
- Example tests now run as part of `cargo test --examples`
- Comprehensive test coverage for critical areas:
  - Kernel view tests (dirty tracking, read/write behavior)
  - Host view tests (dirty marking, access policy enforcement)
  - Staging buffer tests (overlay logic, capacity limits)
  - Staged view tests (commit behavior, access policies)
  - Storage tests (`load_defaults` initialization)
- Shared `test_support` module for DRY test helpers
- Documentation for all public types, traits, and methods
- Documentation for generic parameters on `ShadowStorageBase`
- Expanded example in lib.rs and README showing typical usage patterns

### Changed

- Renamed `action()` to `commit()` on `HostViewStaged` (`action` is now deprecated)

## [0.1.1] - 2026-01-09

### Changed

- Made `KernelShadow::with_view_unchecked` unsafe
- Added `critical_section` example

## [0.1.0] - 2026-01-08

### Added

- Initial release
- Zero-allocation shadow register table with const generics
- Block-based dirty tracking with bitmap
- Dual access patterns: `HostShadow` (application) and `KernelShadow` (hardware driver)
- `HostView` for host writes that mark dirty and trigger persistence
- `KernelView` for kernel reads/writes without dirty marking
- Transactional staged writes with `HostViewStaged` and `PatchStagingBuffer`
- Configurable access policies via `AccessPolicy` trait
- Configurable persistence policies via `PersistPolicy` and `PersistTrigger` traits
- Type-safe builder pattern with `ShadowStorageBuilder`
- Critical section support for thread-safe access
- `no_std` compatible for embedded systems
- Examples: basic, staging, persist, access_policy, critical_section, complex
