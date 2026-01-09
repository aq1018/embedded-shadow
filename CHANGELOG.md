# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Comprehensive test coverage for critical areas:
  - Kernel view tests (dirty tracking, read/write behavior)
  - Host view tests (dirty marking, access policy enforcement)
  - Staging buffer tests (overlay logic, capacity limits)
  - Staged view tests (commit behavior, access policies)
  - Storage tests (`load_defaults` initialization)
- Shared `test_support` module for DRY test helpers

## [0.1.1] - 2026-01-09

### Added

- `load_defaults()` and `load_defaults_unchecked()` for initializing shadow table with factory/EEPROM data without marking dirty
- Example tests now run as part of `cargo test --examples`
- Improved documentation in prelude

### Changed

- Renamed `action()` to `commit()` on `HostViewStaged` (`action` is now deprecated)

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
