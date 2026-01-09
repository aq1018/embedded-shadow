//! Staging example: Transactional writes with rollback capability
//!
//! This example demonstrates:
//! - Adding a staging buffer to shadow storage
//! - Staging multiple writes before committing
//! - Reading with overlay to preview staged changes
//! - Atomic commit of all staged writes
//! - Rollback capability by clearing staged writes

#![no_std]

use embedded_shadow::prelude::*;

pub fn main() {
    // Create base shadow storage
    let storage = ShadowStorageBuilder::new()
        .total_size::<512>()
        .block_size::<32>()
        .block_count::<16>()
        .default_access()
        .no_persist()
        .build();

    // Upgrade with staging capability
    // PatchStagingBuffer<DATA_CAPACITY, MAX_ENTRIES>
    // - 256 bytes total for staged data
    // - Up to 16 separate write operations
    let staging_buffer = PatchStagingBuffer::<256, 16>::new();
    let staged_storage = storage.with_staging(staging_buffer);

    let host = staged_storage.host_shadow();

    // ========== Example 1: Preview Changes Before Commit ==========
    host.with_view(|view| {
        // Write initial data directly to shadow
        view.write_range(0x00, &[0xAA; 8]).unwrap();

        // Stage some changes (not committed yet)
        view.write_range_staged(0x00, &[0x11, 0x22, 0x33, 0x44])
            .unwrap();
        view.write_range_staged(0x10, &[0x55, 0x66, 0x77, 0x88])
            .unwrap();

        // Read with overlay - sees staged changes overlaid on base data
        let mut buffer = [0u8; 8];
        view.read_range_overlay(0x00, &mut buffer).unwrap();
        assert_eq!(&buffer[0..4], &[0x11, 0x22, 0x33, 0x44]); // Staged data
        assert_eq!(&buffer[4..8], &[0xAA; 4]); // Original data

        // Regular read still sees original data
        view.read_range(0x00, &mut buffer).unwrap();
        assert_eq!(buffer, [0xAA; 8]); // All original

        // Commit staged changes
        view.action().unwrap();

        // Now regular read sees the changes
        view.read_range(0x00, &mut buffer).unwrap();
        assert_eq!(&buffer[0..4], &[0x11, 0x22, 0x33, 0x44]); // Now committed
    });

    // ========== Example 2: Rollback Uncommitted Changes ==========
    host.with_view(|view| {
        // Stage multiple register updates
        view.write_range_staged(0x100, &[0x01, 0x02]).unwrap();
        view.write_range_staged(0x102, &[0x03, 0x04]).unwrap();
        view.write_range_staged(0x104, &[0x05, 0x06]).unwrap();

        // Verify staged data exists
        let mut buffer = [0u8; 6];
        view.read_range_overlay(0x100, &mut buffer).unwrap();
        assert_eq!(buffer, [0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);

        // Decide not to commit - just exit the view
        // Staged changes are automatically discarded
    });

    // Verify changes were not committed
    host.with_view(|view| {
        let mut buffer = [0u8; 6];
        view.read_range(0x100, &mut buffer).unwrap();
        assert_eq!(buffer, [0u8; 6]); // Still zeros, changes were discarded
    });

    // ========== Example 3: Overlapping Staged Writes ==========
    host.with_view(|view| {
        // Write base data
        view.write_range(0x180, &[0xFF; 16]).unwrap();

        // Stage overlapping writes (later writes override earlier ones)
        view.write_range_staged(0x180, &[0x01, 0x02, 0x03, 0x04])
            .unwrap();
        view.write_range_staged(0x182, &[0xAA, 0xBB]).unwrap(); // Overlaps previous

        // Read with overlay shows the combined result
        let mut buffer = [0u8; 8];
        view.read_range_overlay(0x180, &mut buffer).unwrap();
        assert_eq!(&buffer[0..2], &[0x01, 0x02]); // From first staged write
        assert_eq!(&buffer[2..4], &[0xAA, 0xBB]); // From second staged write (override)
        assert_eq!(&buffer[4..8], &[0xFF; 4]); // Original data

        // Commit all staged changes
        view.action().unwrap();
    });

    // ========== Example 4: Staging Buffer Limits ==========
    host.with_view(|view| {
        // Try to stage more than buffer capacity
        let mut total_staged = 0;

        // Stage writes until we hit capacity
        for i in 0..20 {
            let addr = (i * 16) as u16;
            let data = [(i + 1) as u8; 8];

            match view.write_range_staged(addr, &data) {
                Ok(()) => total_staged += 1,
                Err(ShadowError::StageFull) => break,
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }

        // We should have staged some but not all
        assert!(total_staged > 0);
        assert!(total_staged < 20);

        // Clear staged to make room
        // (In this case, we exit view without committing)
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_staging_example() {
        super::main();
    }
}
