//! Basic example: Shadow register fundamentals
//!
//! This example demonstrates:
//! - Using the builder pattern to create shadow storage
//! - Host view for application writes
//! - Kernel view for hardware synchronization
//! - Dirty tracking between host and kernel

#![no_std]

use embedded_shadow::prelude::*;

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
    // The host writes application data to the shadow registers
    host_shadow.with_view(|view| {
        // Write some configuration data at address 0x100
        let config_data = [0xDE, 0xAD, 0xBE, 0xEF];
        view.write_range(0x100, &config_data).unwrap();

        // Write some control registers at address 0x200
        let control_data = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        view.write_range(0x200, &control_data).unwrap();

        // These writes automatically mark the affected blocks as dirty
    });

    // ========== Kernel Side Operations ==========
    // The kernel syncs dirty data to hardware
    kernel_shadow.with_view(|view| {
        // Check what's dirty before processing
        assert!(view.is_dirty(0x100, 4).unwrap(), "Config should be dirty");
        assert!(view.is_dirty(0x200, 8).unwrap(), "Control should be dirty");

        // Read back the data
        let mut buffer = [0u8; 8];
        view.read_range(0x200, &mut buffer).unwrap();
        assert_eq!(buffer, [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);

        // Process all dirty blocks (typically sync to hardware)
        let mut blocks_processed = 0;
        view.for_each_dirty_block(|addr, data| {
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
        view.clear_dirty();

        // Verify nothing is dirty anymore
        assert!(!view.any_dirty());
        assert!(!view.is_dirty(0x100, 4).unwrap());
    });

    // ========== Direct Access Without Critical Section ==========
    // If you know there's no concurrent access, use unchecked for performance
    unsafe {
        host_shadow.with_view_unchecked(|view| {
            // This skips the critical section overhead
            view.write_range(0x300, &[0xFF; 16]).unwrap();
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
