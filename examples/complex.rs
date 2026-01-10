//! Complex example: Real-world embedded system simulation
//!
//! This simulates a motor controller with:
//! - Protected bootloader and calibration regions
//! - Staged parameter updates with validation
//! - Flash persistence for configuration
//! - Real-time control registers
//! - Telemetry data collection
//!
//! Demonstrates using typed slice primitives (write_u16_le_at, read_i16_le_at, etc.)
//! to read and write structured data efficiently.

#![no_std]

extern crate heapless;

use embedded_shadow::prelude::*;
use heapless::Vec;

// ============ Memory Map ============
// 0x0000-0x00FF: Bootloader (read-only)
// 0x0100-0x01FF: Calibration data (factory-set, rarely changed)
// 0x0200-0x02FF: User configuration (persistent)
//   - 0x200-0x201: max_speed (u16)
//   - 0x202-0x203: acceleration (u16)
//   - 0x204-0x205: deceleration (u16)
//   - 0x206-0x207: pid_p (u16)
//   - 0x208-0x209: pid_i (u16)
//   - 0x20A-0x20B: pid_d (u16)
// 0x0300-0x03FF: Control registers (real-time)
//   - 0x300-0x301: speed_setpoint (u16)
//   - 0x302-0x303: current_speed (u16)
//   - 0x304-0x305: temperature (i16)
//   - 0x306-0x309: error_count (u32)
// 0x0400-0x07FF: Telemetry buffer (circular, volatile)
//   Each entry (8 bytes): current_speed (u16) | temperature (i16) | error_count (u32)

/// Complex access policy for motor controller
struct MotorControllerPolicy {
    calibration_unlocked: bool,
}

impl MotorControllerPolicy {
    fn new() -> Self {
        Self {
            calibration_unlocked: false,
        }
    }

    fn unlock_calibration(&mut self) {
        self.calibration_unlocked = true;
    }
}

impl AccessPolicy for MotorControllerPolicy {
    fn can_read(&self, _addr: u16, _len: usize) -> bool {
        true // All regions readable
    }

    fn can_write(&self, addr: u16, len: usize) -> bool {
        let end = addr.saturating_add(len as u16);

        match addr {
            // Bootloader - never writable
            0x0000..=0x00FF => false,

            // Calibration - only if unlocked
            0x0100..=0x01FF => self.calibration_unlocked && end <= 0x0200,

            // User config - always writable
            0x0200..=0x02FF => end <= 0x0300,

            // Control registers - always writable
            0x0300..=0x03FF => end <= 0x0400,

            // Telemetry - always writable
            0x0400..=0x07FF => end <= 0x0800,

            _ => false,
        }
    }
}

/// Persistence policy for motor controller
struct MotorPersistPolicy;

impl PersistPolicy<PersistKey> for MotorPersistPolicy {
    fn push_persist_keys_for_range<F>(&self, addr: u16, len: usize, mut push_key: F) -> bool
    where
        F: FnMut(PersistKey),
    {
        let end = addr.saturating_add(len as u16);

        // Check if write touches any persistent region
        let mut needs_persist = false;

        // Calibration data (if touched, save it)
        if addr < 0x0200 && end > 0x0100 {
            push_key(PersistKey::Calibration);
            needs_persist = true;
        }

        // User configuration (if touched, save it)
        if addr < 0x0300 && end > 0x0200 {
            push_key(PersistKey::UserConfig);
            needs_persist = true;
        }

        // Control and telemetry are volatile, don't persist

        needs_persist
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PersistKey {
    Calibration,
    UserConfig,
}

/// Smart persist trigger with wear leveling awareness
struct SmartPersistTrigger {
    pending: Vec<PersistKey, 4>,
    calibration_write_count: u32,
    config_write_count: u32,
}

impl SmartPersistTrigger {
    fn new() -> Self {
        Self {
            pending: Vec::new(),
            calibration_write_count: 0,
            config_write_count: 0,
        }
    }
}

impl PersistTrigger<PersistKey> for SmartPersistTrigger {
    fn push_key(&mut self, key: PersistKey) {
        if !self.pending.contains(&key) {
            let _ = self.pending.push(key);
        }
    }

    fn request_persist(&mut self) {
        for key in self.pending.iter() {
            match key {
                PersistKey::Calibration => {
                    // Calibration is rarely written, persist immediately
                    self.calibration_write_count += 1;
                    // flash.write_calibration_page(data);
                }
                PersistKey::UserConfig => {
                    // User config might change often, batch writes
                    self.config_write_count += 1;
                    if self.config_write_count % 10 == 0 {
                        // Only persist every 10th change to reduce wear
                        // flash.write_config_page(data);
                    }
                }
            }
        }
        self.pending.clear();
    }
}

/// Motor controller state for the example
struct MotorController {
    speed_setpoint: u16,
    current_speed: u16,
    temperature: i16,
    error_count: u32,
}

pub fn main() {
    // Create the complete motor controller shadow system
    let mut access_policy = MotorControllerPolicy::new();

    // Note: In real code, we'd need to pass access_policy by value,
    // but for demo we'll create a new one
    let storage = ShadowStorageBuilder::new()
        .total_size::<2048>() // 2KB total
        .block_size::<64>() // 64-byte blocks
        .block_count::<32>() // 32 blocks
        .access_policy(MotorControllerPolicy::new())
        .persist_policy(MotorPersistPolicy)
        .persist_trigger(SmartPersistTrigger::new())
        .build();

    // Add staging for atomic parameter updates
    let staging_buffer = PatchStagingBuffer::<256, 16>::new();
    let staged_storage = storage.with_staging(staging_buffer);

    let host = staged_storage.host_shadow();
    let kernel = staged_storage.kernel_shadow();

    // ========== Initialize System ==========
    host.with_view(|view| {
        // Load default user configuration using typed primitives
        // Layout: max_speed (u16) | acceleration (u16) | deceleration (u16) | pid_p (u16) | pid_i (u16) | pid_d (u16)
        view.with_wo_slice(0x200, 12, |mut slice| {
            slice.write_u16_le_at(0, 256); // max_speed: 256 RPM
            slice.write_u16_le_at(2, 100); // acceleration: 100 units
            slice.write_u16_le_at(4, 50); // deceleration: 50 units
            slice.write_u16_le_at(6, 256); // pid_p: 1.0 (scaled)
            slice.write_u16_le_at(8, 512); // pid_i: 2.0 (scaled)
            slice.write_u16_le_at(10, 256); // pid_d: 1.0 (scaled)
            WriteResult::Dirty(())
        })
        .unwrap();

        // Initialize control registers to zero
        view.with_wo_slice(0x300, 10, |mut slice| {
            slice.write_u16_le_at(0, 0); // speed_setpoint
            slice.write_u16_le_at(2, 0); // current_speed
            slice.write_i16_le_at(4, 0); // temperature
            slice.write_u32_le_at(6, 0); // error_count
            WriteResult::Dirty(())
        })
        .unwrap();
    });

    // ========== Runtime Operation ==========
    let mut controller = MotorController {
        speed_setpoint: 0,
        current_speed: 0,
        temperature: 25,
        error_count: 0,
    };

    // Simulate parameter update with validation using typed primitives
    host.with_view(|view| {
        // Stage new PID parameters using typed writes
        view.alloc_staged(0x206, 2, |mut slice| {
            slice.write_u16_le_at(0, 768); // New P: 3.0 (scaled by 256)
            WriteResult::Dirty(())
        })
        .unwrap();

        view.alloc_staged(0x208, 2, |mut slice| {
            slice.write_u16_le_at(0, 1024); // New I: 4.0 (scaled by 256)
            WriteResult::Dirty(())
        })
        .unwrap();

        view.alloc_staged(0x20A, 2, |mut slice| {
            slice.write_u16_le_at(0, 512); // New D: 2.0 (scaled by 256)
            WriteResult::Dirty(())
        })
        .unwrap();

        // Validate staged parameters (in real system, would read from staging)
        let p_scaled = 768u16;
        let p = p_scaled / 256; // Convert back to actual value

        if p <= 10 {
            // Validation passed - commit staged changes atomically
            view.commit_staged().unwrap();
        }
        // Otherwise staged changes are discarded
    });

    // Simulate control loop
    for cycle in 0..10 {
        // Host updates control registers using typed slice primitives
        host.with_view(|view| {
            controller.speed_setpoint = 100 + cycle * 10;

            // Write all control fields at once using typed primitives
            // Layout: speed_setpoint (u16) | current_speed (u16) | temperature (i16) | error_count (u32)
            view.with_wo_slice(0x300, 10, |mut slice| {
                slice.write_u16_le_at(0, controller.speed_setpoint);
                slice.write_u16_le_at(2, controller.current_speed);
                slice.write_i16_le_at(4, controller.temperature);
                slice.write_u32_le_at(6, controller.error_count);
                WriteResult::Dirty(())
            })
            .unwrap();

            // Update telemetry buffer using typed writes
            // Each entry (8 bytes): current_speed (u16) | temperature (i16) | error_count (u32)
            let telemetry_offset = 0x400 + cycle * 8;
            view.with_wo_slice(telemetry_offset, 8, |mut slice| {
                slice.write_u16_le_at(0, controller.current_speed);
                slice.write_i16_le_at(2, controller.temperature);
                slice.write_u32_le_at(4, controller.error_count);
                WriteResult::Dirty(())
            })
            .unwrap();
        });

        // Kernel syncs to hardware
        kernel.with_view(|view| {
            // Process control registers if dirty
            if view.is_dirty(0x300, 10).unwrap() {
                // Read control data using typed primitives
                view.with_ro_slice(0x300, 10, |slice| {
                    let setpoint = slice.read_u16_le_at(0);
                    let current = slice.read_u16_le_at(2);
                    let temp = slice.read_i16_le_at(4);
                    let errors = slice.read_u32_le_at(6);

                    // In real system, would write to motor driver hardware:
                    // motor_driver.set_setpoint(setpoint);
                    let _ = (setpoint, current, temp, errors);
                })
                .unwrap();

                view.clear_all_dirty();
            }
        });

        // Simulate motor response
        controller.current_speed =
            controller.current_speed * 9 / 10 + controller.speed_setpoint / 10;
        controller.temperature += 1;
    }

    // ========== Calibration Mode ==========
    // Special sequence to update calibration
    access_policy.unlock_calibration();

    // Now we can update calibration (in real system, would recreate storage with unlocked policy)
    let storage_unlocked = ShadowStorageBuilder::new()
        .total_size::<2048>()
        .block_size::<64>()
        .block_count::<32>()
        .access_policy(access_policy)
        .persist_policy(MotorPersistPolicy)
        .persist_trigger(SmartPersistTrigger::new())
        .build();

    let host_unlocked = storage_unlocked.host_shadow();

    host_unlocked.with_view(|view| {
        // Update motor calibration constants using typed primitives
        // Layout: motor_constant (u16) | offset (u16) | scale_factor (u16)
        view.with_wo_slice(0x100, 6, |mut slice| {
            slice.write_u16_le_at(0, 1023); // motor_constant
            slice.write_u16_le_at(2, 32); // offset
            slice.write_u16_le_at(4, 16); // scale_factor
            WriteResult::Dirty(())
        })
        .unwrap();
        // This triggers immediate persistence due to calibration policy
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_complex_example() {
        super::main();
    }
}
