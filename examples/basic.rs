//! Basic example: Shadow register fundamentals
//!
//! This example demonstrates:
//! - Using the builder pattern to create shadow storage
//! - Host view for application writes
//! - Kernel view for hardware synchronization
//! - Dirty tracking between host and kernel
//! - Using typed slice primitives (read_u16_le_at, write_u32_le_at, etc.)

#![no_std]

use embedded_shadow::prelude::*;

// ============ Register Layout ============
// Define register structures using constants. In embedded systems, registers
// typically have fixed layouts - we document them here and use typed slice
// primitives to access individual fields.

/// Configuration register at 0x100
/// Layout: flags (u16) | timeout_ms (u16)
const CONFIG_ADDR: u16 = 0x100;
const CONFIG_SIZE: usize = 4;

/// Control register at 0x200
/// Layout: mode (u8) | speed (u16) | direction (u8) | position (u32)
const CONTROL_ADDR: u16 = 0x200;
const CONTROL_SIZE: usize = 8;

pub fn main() {
    // Create a 1KB shadow register table with 64-byte blocks
    // Total size must equal block_size * block_count
    let storage = ShadowStorageBuilder::new()
        .total_size::<1024>() // Total storage size in bytes
        .block_size::<64>() // Size of each dirty-tracking block
        .block_count::<16>() // Number of blocks (1024 / 64 = 16)
        .default_access() // Allow all reads and writes
        .no_persist() // No persistence needed for this example
        .build();

    // Get separate views for host (application) and kernel (hardware driver)
    let host_shadow = storage.host_shadow();
    let kernel_shadow = storage.kernel_shadow();

    // ========== Host Side Operations ==========
    // The host writes application data to the shadow registers using typed primitives
    host_shadow.with_view(|view| {
        // Write configuration register using typed slice primitives
        // Layout: flags (u16) | timeout_ms (u16)
        view.with_wo_slice(CONFIG_ADDR, CONFIG_SIZE, |mut slice| {
            slice.write_u16_le_at(0, 0x001F); // flags: enable all features
            slice.write_u16_le_at(2, 5000); // timeout: 5000ms
            WriteResult::Dirty(())
        })
        .unwrap();

        // Write control register with mixed field sizes
        // Layout: mode (u8) | speed (u16) | direction (u8) | position (u32)
        view.with_wo_slice(CONTROL_ADDR, CONTROL_SIZE, |mut slice| {
            slice.write_u8_at(0, 0x02); // mode: run
            slice.write_u16_le_at(1, 1500); // speed: 1500 RPM
            slice.write_u8_at(3, 0x01); // direction: forward
            slice.write_u32_le_at(4, 0); // position: 0
            WriteResult::Dirty(())
        })
        .unwrap();

        // These writes automatically mark the affected blocks as dirty
    });

    // ========== Kernel Side Operations ==========
    // The kernel syncs dirty data to hardware
    kernel_shadow.with_view(|view| {
        // Check what's dirty before processing
        assert!(
            view.is_dirty(CONFIG_ADDR, CONFIG_SIZE).unwrap(),
            "Config should be dirty"
        );
        assert!(
            view.is_dirty(CONTROL_ADDR, CONTROL_SIZE).unwrap(),
            "Control should be dirty"
        );

        // Read back control register using typed slice primitives
        view.with_ro_slice(CONTROL_ADDR, CONTROL_SIZE, |slice| {
            let mode = slice.read_u8_at(0);
            let speed = slice.read_u16_le_at(1);
            let direction = slice.read_u8_at(3);
            let position = slice.read_u32_le_at(4);

            assert_eq!(mode, 0x02);
            assert_eq!(speed, 1500);
            assert_eq!(direction, 0x01);
            assert_eq!(position, 0);
        })
        .unwrap();

        // Process all dirty blocks (typically sync to hardware)
        let mut blocks_processed = 0;
        view.iter_dirty(|addr, data| {
            // In a real system, this is where you'd write to hardware:
            // unsafe { hardware_registers.write(addr, data); }

            // For demo, just count the blocks
            blocks_processed += 1;

            // Log which blocks are being synced (addresses are block-aligned)
            match addr {
                0x80 => {
                    // Block containing 0x100 (blocks are 64 bytes, so 0x80-0xBF)
                    assert_eq!(data.len(), 64);
                }
                0x200 => {
                    // Block containing 0x200 (0x200-0x23F)
                    assert_eq!(data.len(), 64);
                }
                _ => {}
            }

            Ok(())
        })
        .unwrap();

        assert_eq!(blocks_processed, 2, "Should process 2 dirty blocks");

        // After syncing to hardware, clear the dirty flags
        view.clear_all_dirty();

        // Verify nothing is dirty anymore
        assert!(!view.any_dirty());
        assert!(!view.is_dirty(CONFIG_ADDR, CONFIG_SIZE).unwrap());
    });

    // ========== Direct Access Without Critical Section ==========
    // If you know there's no concurrent access, use unchecked for performance
    unsafe {
        host_shadow.with_view_unchecked(|view| {
            // This skips the critical section overhead
            // Write a status register using typed primitives
            view.with_wo_slice(0x300, 8, |mut slice| {
                slice.write_u32_le_at(0, 0xDEADBEEF); // status code
                slice.write_u16_le_at(4, 42); // sequence number
                slice.write_u16_le_at(6, 0x8000); // flags
                WriteResult::Dirty(())
            })
            .unwrap();
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_basic_example() {
        super::main();
    }
}
