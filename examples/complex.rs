//! Complex example: Real-world embedded system simulation
//! 
//! This simulates a motor controller with:
//! - Protected bootloader and calibration regions
//! - Staged parameter updates with validation
//! - Flash persistence for configuration
//! - Real-time control registers
//! - Telemetry data collection

#![no_std]

extern crate heapless;

use embedded_shadow::prelude::*;
use heapless::Vec;

// ============ Memory Map ============
// 0x0000-0x00FF: Bootloader (read-only)
// 0x0100-0x01FF: Calibration data (factory-set, rarely changed)
// 0x0200-0x02FF: User configuration (persistent)
// 0x0300-0x03FF: Control registers (real-time)
// 0x0400-0x07FF: Telemetry buffer (circular, volatile)

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
        .total_size::<2048>()  // 2KB total
        .block_size::<64>()    // 64-byte blocks
        .block_count::<32>()   // 32 blocks
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
        // Load default user configuration
        let default_config = [
            0x01, 0x00,  // Max speed: 256 RPM
            0x64, 0x00,  // Acceleration: 100 units
            0x32, 0x00,  // Deceleration: 50 units
            0x00, 0x01,  // PID P: 1.0
            0x00, 0x02,  // PID I: 2.0
            0x00, 0x01,  // PID D: 1.0
        ];
        view.write_range(0x200, &default_config).unwrap();
        
        // Initialize control registers
        view.write_range(0x300, &[0x00; 16]).unwrap(); // All stop
    });
    
    // ========== Runtime Operation ==========
    let mut controller = MotorController {
        speed_setpoint: 0,
        current_speed: 0,
        temperature: 25,
        error_count: 0,
    };
    
    // Simulate parameter update with validation
    host.with_view(|view| {
        // Stage new PID parameters
        view.write_range_staged(0x206, &[0x00, 0x03]).unwrap(); // New P: 3.0
        view.write_range_staged(0x208, &[0x00, 0x04]).unwrap(); // New I: 4.0
        view.write_range_staged(0x20A, &[0x00, 0x02]).unwrap(); // New D: 2.0
        
        // Validate staged parameters
        let mut p_value = [0u8; 2];
        view.read_range_overlay(0x206, &mut p_value).unwrap();
        let p = u16::from_le_bytes(p_value);
        
        if p <= 10 {  // Validation passed
            // Commit the staged changes
            view.action().unwrap();
        }
        // Otherwise staged changes are discarded
    });
    
    // Simulate control loop
    for cycle in 0..10 {
        // Host updates control registers
        host.with_view(|view| {
            controller.speed_setpoint = 100 + cycle * 10;
            let setpoint_bytes = controller.speed_setpoint.to_le_bytes();
            view.write_range(0x300, &setpoint_bytes).unwrap();
            
            // Update telemetry
            let telemetry_offset = 0x400 + (cycle * 8) as u16;
            let telemetry = [
                controller.current_speed.to_le_bytes()[0],
                controller.current_speed.to_le_bytes()[1],
                controller.temperature.to_le_bytes()[0],
                controller.temperature.to_le_bytes()[1],
                controller.error_count.to_le_bytes()[0],
                controller.error_count.to_le_bytes()[1],
                controller.error_count.to_le_bytes()[2],
                controller.error_count.to_le_bytes()[3],
            ];
            view.write_range(telemetry_offset, &telemetry).unwrap();
        });
        
        // Kernel syncs to hardware
        kernel.with_view(|view| {
            // Process control registers
            if view.is_dirty(0x300, 16).unwrap() {
                view.for_each_dirty_block(|addr, data| {
                    if addr == 0x300 {
                        // Write to motor driver hardware
                        // motor_driver.set_registers(data);
                        let _ = data;
                    }
                    Ok(())
                }).unwrap();
                
                view.clear_dirty();
            }
        });
        
        // Simulate motor response
        controller.current_speed = controller.current_speed * 9 / 10 + controller.speed_setpoint / 10;
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
        // Update motor calibration constants
        let calibration = [
            0xFF, 0x03,  // Motor constant
            0x20, 0x00,  // Offset
            0x10, 0x00,  // Scale factor
        ];
        view.write_range(0x100, &calibration).unwrap();
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