//! Persistence example: Managing what and when to persist
//!
//! This example demonstrates:
//! - Custom persist policies to determine what needs saving
//! - Custom persist triggers to batch and execute saves
//! - Different persistence strategies (immediate, batched, periodic)
//! - Integration with flash memory patterns

#![no_std]

extern crate heapless;

use embedded_shadow::prelude::*;
use heapless::Vec;

/// Persist policy that maps address ranges to flash sectors
struct FlashSectorPolicy {
    sector_size: u16,
}

impl FlashSectorPolicy {
    const fn new(sector_size: u16) -> Self {
        Self { sector_size }
    }
}

impl PersistPolicy<u16> for FlashSectorPolicy {
    fn push_persist_keys_for_range<F>(&self, addr: u16, len: usize, mut push_key: F) -> bool
    where
        F: FnMut(u16),
    {
        // Calculate which sectors are affected
        let start_sector = addr / self.sector_size;
        let end_addr = addr + len as u16;
        let end_sector = end_addr.div_ceil(self.sector_size);

        // Push each affected sector as a persist key
        for sector in start_sector..end_sector {
            push_key(sector * self.sector_size); // Use sector base address as key
        }

        // Return true to request persistence
        true
    }
}

/// Persist policy for configuration registers that always persist immediately
struct CriticalRegisterPolicy;

impl PersistPolicy<u16> for CriticalRegisterPolicy {
    fn push_persist_keys_for_range<F>(&self, addr: u16, len: usize, mut push_key: F) -> bool
    where
        F: FnMut(u16),
    {
        // Critical registers are at 0x00-0x1F
        const CRITICAL_START: u16 = 0x00;
        const CRITICAL_END: u16 = 0x20;

        let end_addr = addr + len as u16;

        // Check if this write touches critical registers
        if addr < CRITICAL_END && end_addr > CRITICAL_START {
            // Push the exact range as key for precise persistence
            push_key(addr);
            true // Request immediate persistence
        } else {
            false // Non-critical, don't persist yet
        }
    }
}

/// Persist trigger that collects sectors and batches writes
struct BatchedFlashTrigger {
    pending_sectors: Vec<u16, 16>,
    write_count: usize,
    batch_size: usize,
}

impl BatchedFlashTrigger {
    fn new(batch_size: usize) -> Self {
        Self {
            pending_sectors: Vec::new(),
            write_count: 0,
            batch_size,
        }
    }

    fn do_persist(&mut self) {
        // In real code, this would write to flash
        // For demo, we just track what would be written
        if !self.pending_sectors.is_empty() {
            // Simulate flash write
            for sector in self.pending_sectors.iter() {
                // flash_driver.erase_sector(*sector);
                // flash_driver.write_sector(*sector, data);
                let _ = sector; // Suppress unused warning
            }

            self.pending_sectors.clear();
            self.write_count = 0;
        }
    }
}

impl PersistTrigger<u16> for BatchedFlashTrigger {
    fn push_key(&mut self, sector_addr: u16) {
        // Add sector if not already pending
        if !self.pending_sectors.contains(&sector_addr) {
            let _ = self.pending_sectors.push(sector_addr);
        }

        self.write_count += 1;
    }

    fn request_persist(&mut self) {
        // Only persist if we've accumulated enough writes
        if self.write_count >= self.batch_size {
            self.do_persist();
        }
        // Otherwise wait for more writes to batch
    }
}

/// Simple immediate trigger for critical data
struct ImmediatePersistTrigger {
    last_persisted_addr: Option<u16>,
}

impl ImmediatePersistTrigger {
    fn new() -> Self {
        Self {
            last_persisted_addr: None,
        }
    }
}

impl PersistTrigger<u16> for ImmediatePersistTrigger {
    fn push_key(&mut self, addr: u16) {
        self.last_persisted_addr = Some(addr);
    }

    fn request_persist(&mut self) {
        if let Some(addr) = self.last_persisted_addr {
            // Immediately persist critical data
            // flash_driver.write_immediate(addr, data);
            let _ = addr; // Suppress unused warning
            self.last_persisted_addr = None;
        }
    }
}

pub fn main() {
    example_flash_sectors();
    example_critical_registers();
    example_selective_persistence();
}

fn example_flash_sectors() {
    // Setup with 4KB flash sectors
    let storage = ShadowStorageBuilder::new()
        .total_size::<16384>() // 16KB total
        .block_size::<256>() // 256-byte dirty blocks
        .block_count::<64>() // 64 blocks
        .default_access()
        .persist_policy(FlashSectorPolicy::new(4096)) // 4KB sectors
        .persist_trigger(BatchedFlashTrigger::new(4)) // Batch 4 writes
        .build();

    let host = storage.host_shadow();

    host.with_view(|view| {
        // Small writes accumulate
        view.with_wo_slice(0x100, 32, |mut slice| {
            slice.fill(0x01);
            (true, ())
        })
        .unwrap(); // Sector 0

        view.with_wo_slice(0x200, 32, |mut slice| {
            slice.fill(0x02);
            (true, ())
        })
        .unwrap(); // Sector 0

        view.with_wo_slice(0x300, 32, |mut slice| {
            slice.fill(0x03);
            (true, ())
        })
        .unwrap(); // Sector 0

        // Fourth write triggers batch persistence
        view.with_wo_slice(0x1000, 32, |mut slice| {
            slice.fill(0x04);
            (true, ())
        })
        .unwrap(); // Sector 1
        // BatchedFlashTrigger would now persist sectors 0 and 1

        // Write to another sector
        view.with_wo_slice(0x2000, 32, |mut slice| {
            slice.fill(0x05);
            (true, ())
        })
        .unwrap(); // Sector 2
        // Not persisted yet, waiting for more writes
    });
}

fn example_critical_registers() {
    let storage = ShadowStorageBuilder::new()
        .total_size::<256>()
        .block_size::<32>()
        .block_count::<8>()
        .default_access()
        .persist_policy(CriticalRegisterPolicy)
        .persist_trigger(ImmediatePersistTrigger::new())
        .build();

    let host = storage.host_shadow();

    host.with_view(|view| {
        // Write to critical register - persists immediately
        view.with_wo_slice(0x10, 4, |mut slice| {
            slice.fill(0xFF);
            (true, ())
        })
        .unwrap();
        // ImmediatePersistTrigger executes right away

        // Write to non-critical area - not persisted
        view.with_wo_slice(0x80, 4, |mut slice| {
            slice.fill(0xAA);
            (true, ())
        })
        .unwrap();
        // No persistence triggered

        // Another critical write - persists immediately
        view.with_wo_slice(0x00, 2, |mut slice| {
            slice.copy_from_slice(&[0x12, 0x34]);
            (true, ())
        })
        .unwrap();
        // ImmediatePersistTrigger executes again
    });
}

fn example_selective_persistence() {
    /// Policy that only persists configuration blocks
    struct ConfigOnlyPolicy;

    impl PersistPolicy<&'static str> for ConfigOnlyPolicy {
        fn push_persist_keys_for_range<F>(&self, addr: u16, _len: usize, mut push_key: F) -> bool
        where
            F: FnMut(&'static str),
        {
            // Define configuration regions
            match addr {
                0x000..=0x0FF => {
                    push_key("boot_config");
                    true
                }
                0x100..=0x1FF => {
                    push_key("app_config");
                    true
                }
                0x200..=0x2FF => {
                    push_key("user_settings");
                    true
                }
                _ => false, // Don't persist other regions
            }
        }
    }

    /// Trigger that groups by configuration type
    struct ConfigGroupTrigger {
        pending: Vec<&'static str, 8>,
    }

    impl ConfigGroupTrigger {
        fn new() -> Self {
            Self {
                pending: Vec::new(),
            }
        }
    }

    impl PersistTrigger<&'static str> for ConfigGroupTrigger {
        fn push_key(&mut self, config_type: &'static str) {
            if !self.pending.contains(&config_type) {
                let _ = self.pending.push(config_type);
            }
        }

        fn request_persist(&mut self) {
            // Persist each configuration type
            for config in self.pending.iter() {
                match *config {
                    "boot_config" => {
                        // Save boot configuration to protected flash
                    }
                    "app_config" => {
                        // Save application config to main flash
                    }
                    "user_settings" => {
                        // Save user settings to EEPROM
                    }
                    _ => {}
                }
            }
            self.pending.clear();
        }
    }

    let storage = ShadowStorageBuilder::new()
        .total_size::<1024>()
        .block_size::<64>()
        .block_count::<16>()
        .default_access()
        .persist_policy(ConfigOnlyPolicy)
        .persist_trigger(ConfigGroupTrigger::new())
        .build();

    let host = storage.host_shadow();

    host.with_view(|view| {
        // Write to config area - will persist
        view.with_wo_slice(0x050, 8, |mut slice| {
            slice.fill(0x11);
            (true, ())
        })
        .unwrap(); // boot_config

        // Write to data area - won't persist
        view.with_wo_slice(0x300, 8, |mut slice| {
            slice.fill(0x22);
            (true, ())
        })
        .unwrap(); // Not config

        // Write to different config - will persist separately
        view.with_wo_slice(0x150, 8, |mut slice| {
            slice.fill(0x33);
            (true, ())
        })
        .unwrap(); // app_config

        // ConfigGroupTrigger will persist boot_config and app_config
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_persist_example() {
        super::main();
    }
}
