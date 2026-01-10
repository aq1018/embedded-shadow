//! Staging example: Transactional writes with rollback capability
//!
//! This example demonstrates:
//! - Adding a staging buffer to shadow storage
//! - Staging multiple writes before committing
//! - Atomic commit of all staged writes
//! - Rollback capability by clearing staged writes
//! - Using typed slice primitives with structured data

#![no_std]

use embedded_shadow::prelude::*;

// ============ Register Layout ============
// PID controller parameters (staged atomically)
// Layout: p_gain (u16) | i_gain (u16) | d_gain (u16) | output_limit (u16)
const PID_ADDR: u16 = 0x100;
const PID_SIZE: usize = 8;

// Sensor calibration (staged atomically)
// Layout: offset (i16) | scale (u16) | min_value (i16) | max_value (i16)
const SENSOR_CAL_ADDR: u16 = 0x180;
const SENSOR_CAL_SIZE: usize = 8;

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

    // ========== Example 1: Stage and Commit with Typed Primitives ==========
    host.with_view(|view| {
        // Initialize PID parameters directly
        view.with_wo_slice(PID_ADDR, PID_SIZE, |mut slice| {
            slice.write_u16_le_at(0, 100); // p_gain
            slice.write_u16_le_at(2, 50); // i_gain
            slice.write_u16_le_at(4, 25); // d_gain
            slice.write_u16_le_at(6, 1000); // output_limit
            WriteResult::Dirty(())
        })
        .unwrap();

        // Stage new PID values atomically (not committed yet)
        view.alloc_staged(PID_ADDR, PID_SIZE, |mut slice| {
            slice.write_u16_le_at(0, 200); // new p_gain
            slice.write_u16_le_at(2, 100); // new i_gain
            slice.write_u16_le_at(4, 50); // new d_gain
            slice.write_u16_le_at(6, 2000); // new output_limit
            WriteResult::Dirty(())
        })
        .unwrap();

        // Regular read still sees original data (staged changes not yet applied)
        view.with_ro_slice(PID_ADDR, PID_SIZE, |slice| {
            assert_eq!(slice.read_u16_le_at(0), 100); // Still original p_gain
            assert_eq!(slice.read_u16_le_at(6), 1000); // Still original output_limit
        })
        .unwrap();

        // Commit staged changes atomically
        view.commit_staged().unwrap();

        // Now regular read sees the new values
        view.with_ro_slice(PID_ADDR, PID_SIZE, |slice| {
            assert_eq!(slice.read_u16_le_at(0), 200); // New p_gain
            assert_eq!(slice.read_u16_le_at(2), 100); // New i_gain
            assert_eq!(slice.read_u16_le_at(4), 50); // New d_gain
            assert_eq!(slice.read_u16_le_at(6), 2000); // New output_limit
        })
        .unwrap();
    });

    // ========== Example 2: Rollback Uncommitted Changes ==========
    host.with_view(|view| {
        // Stage sensor calibration updates using typed primitives
        // Layout: offset (i16) | scale (u16) | min_value (i16) | max_value (i16)
        view.alloc_staged(SENSOR_CAL_ADDR, SENSOR_CAL_SIZE, |mut slice| {
            slice.write_i16_le_at(0, -50); // offset
            slice.write_u16_le_at(2, 1000); // scale
            slice.write_i16_le_at(4, -100); // min_value
            slice.write_i16_le_at(6, 500); // max_value
            WriteResult::Dirty(())
        })
        .unwrap();

        // Simulate validation failure - decide not to commit
        let is_valid = false; // Validation failed!
        if is_valid {
            view.commit_staged().unwrap();
        }
        // Staged changes are automatically discarded when view exits
    });

    // Verify changes were not committed
    host.with_view(|view| {
        view.with_ro_slice(SENSOR_CAL_ADDR, SENSOR_CAL_SIZE, |slice| {
            // All zeros - staged changes were discarded
            assert_eq!(slice.read_i16_le_at(0), 0);
            assert_eq!(slice.read_u16_le_at(2), 0);
        })
        .unwrap();
    });

    // ========== Example 3: Overlapping Staged Writes ==========
    host.with_view(|view| {
        // Initialize sensor calibration with defaults
        view.with_wo_slice(SENSOR_CAL_ADDR, SENSOR_CAL_SIZE, |mut slice| {
            slice.write_i16_le_at(0, 0); // offset
            slice.write_u16_le_at(2, 256); // scale (1.0 in Q8)
            slice.write_i16_le_at(4, 0); // min_value
            slice.write_i16_le_at(6, 100); // max_value
            WriteResult::Dirty(())
        })
        .unwrap();

        // Stage full calibration update
        view.alloc_staged(SENSOR_CAL_ADDR, SENSOR_CAL_SIZE, |mut slice| {
            slice.write_i16_le_at(0, 10); // offset
            slice.write_u16_le_at(2, 512); // scale (2.0 in Q8)
            slice.write_i16_le_at(4, -50); // min_value
            slice.write_i16_le_at(6, 200); // max_value
            WriteResult::Dirty(())
        })
        .unwrap();

        // Stage partial override - just update min/max (overlaps previous)
        view.alloc_staged(SENSOR_CAL_ADDR + 4, 4, |mut slice| {
            slice.write_i16_le_at(0, -100); // override min_value
            slice.write_i16_le_at(2, 500); // override max_value
            WriteResult::Dirty(())
        })
        .unwrap();

        // Commit all staged changes
        view.commit_staged().unwrap();

        // Read back - later staged writes override earlier ones
        view.with_ro_slice(SENSOR_CAL_ADDR, SENSOR_CAL_SIZE, |slice| {
            assert_eq!(slice.read_i16_le_at(0), 10); // From first staged write
            assert_eq!(slice.read_u16_le_at(2), 512); // From first staged write
            assert_eq!(slice.read_i16_le_at(4), -100); // From second staged write (override)
            assert_eq!(slice.read_i16_le_at(6), 500); // From second staged write (override)
        })
        .unwrap();
    });

    // ========== Example 4: Staging Buffer Limits ==========
    host.with_view(|view| {
        // Try to stage more than buffer capacity
        let mut total_staged = 0;

        // Stage writes until we hit capacity
        for i in 0..20 {
            let addr = (i * 16) as u16;

            let result = view.alloc_staged(addr, 8, |mut slice| {
                slice.fill((i + 1) as u8);
                WriteResult::Dirty(())
            });

            match result {
                Ok(_) => total_staged += 1,
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
