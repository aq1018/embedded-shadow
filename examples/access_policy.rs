//! Access Policy example: Controlling read/write permissions
//! 
//! This example demonstrates:
//! - Creating custom access policies
//! - Protecting memory regions from writes
//! - Implementing read-only and write-only regions
//! - Layered security with multiple policies

#![no_std]

use embedded_shadow::prelude::*;

/// Access policy that protects the bootloader region
struct BootloaderProtection;

impl AccessPolicy for BootloaderProtection {
    fn can_read(&self, _addr: u16, _len: usize) -> bool {
        // Allow reading everywhere
        true
    }
    
    fn can_write(&self, addr: u16, len: usize) -> bool {
        // Bootloader is at 0x000-0x0FF (first 256 bytes)
        // Deny writes that would touch this region
        let end = addr.saturating_add(len as u16);
        
        // Allow write only if it doesn't touch bootloader region
        addr >= 0x100 || end == 0
    }
}

/// Access policy for memory-mapped peripheral registers
struct PeripheralAccessPolicy;

impl AccessPolicy for PeripheralAccessPolicy {
    fn can_read(&self, addr: u16, len: usize) -> bool {
        let end = addr.saturating_add(len as u16);
        
        // Define peripheral regions
        const UART_BASE: u16 = 0x400;
        const UART_END: u16 = 0x420;
        const GPIO_BASE: u16 = 0x500;
        const GPIO_END: u16 = 0x540;
        const TIMER_BASE: u16 = 0x600;
        const TIMER_END: u16 = 0x620;
        
        // Check if access is entirely within a peripheral region
        (addr >= UART_BASE && end <= UART_END) ||
        (addr >= GPIO_BASE && end <= GPIO_END) ||
        (addr >= TIMER_BASE && end <= TIMER_END)
    }
    
    fn can_write(&self, addr: u16, len: usize) -> bool {
        // Write-only register at 0x608 (timer clear)
        if addr == 0x608 && len <= 4 {
            return true;
        }
        
        // Otherwise same as read permissions
        self.can_read(addr, len)
    }
}

/// Layered policy that combines multiple policies
struct LayeredPolicy {
    bootloader: BootloaderProtection,
    peripheral: PeripheralAccessPolicy,
}

impl AccessPolicy for LayeredPolicy {
    fn can_read(&self, addr: u16, len: usize) -> bool {
        // Both policies must allow the read
        self.bootloader.can_read(addr, len) && 
        self.peripheral.can_read(addr, len)
    }
    
    fn can_write(&self, addr: u16, len: usize) -> bool {
        // Both policies must allow the write
        self.bootloader.can_write(addr, len) && 
        self.peripheral.can_write(addr, len)
    }
}

pub fn main() {
    example_bootloader_protection();
    example_peripheral_access();
    example_layered_security();
}

fn example_bootloader_protection() {
    let storage = ShadowStorageBuilder::new()
        .total_size::<1024>()
        .block_size::<64>()
        .block_count::<16>()
        .access_policy(BootloaderProtection)
        .no_persist()
        .build();
    
    let host = storage.host_shadow();
    
    host.with_view(|view| {
        // Try to write to bootloader region - should fail
        assert_eq!(
            view.write_range(0x00, &[0xFF; 4]),
            Err(ShadowError::Denied)
        );
        assert_eq!(
            view.write_range(0xFF, &[0xFF; 2]), // Crosses into protected region
            Err(ShadowError::Denied)
        );
        
        // Write to application region - should succeed
        assert!(view.write_range(0x100, &[0xAA; 4]).is_ok());
        
        // Read from bootloader region - should succeed
        let mut buffer = [0u8; 4];
        assert!(view.read_range(0x00, &mut buffer).is_ok());
    });
}

fn example_peripheral_access() {
    let storage = ShadowStorageBuilder::new()
        .total_size::<2048>()
        .block_size::<64>()
        .block_count::<32>()
        .access_policy(PeripheralAccessPolicy)
        .no_persist()
        .build();
    
    let host = storage.host_shadow();
    
    host.with_view(|view| {
        // Access valid peripheral regions
        
        // UART registers (0x400-0x41F)
        assert!(view.write_range(0x400, &[0x55; 4]).is_ok());
        assert!(view.read_range(0x400, &mut [0u8; 4]).is_ok());
        
        // GPIO registers (0x500-0x53F)
        assert!(view.write_range(0x500, &[0xAA; 8]).is_ok());
        
        // Timer registers (0x600-0x61F)
        assert!(view.write_range(0x600, &[0x01, 0x02]).is_ok());
        
        // Try to access non-peripheral memory - should fail
        assert_eq!(
            view.write_range(0x300, &[0xFF; 4]),
            Err(ShadowError::Denied)
        );
        assert_eq!(
            view.read_range(0x300, &mut [0u8; 4]),
            Err(ShadowError::Denied)
        );
        
        // Try to access across peripheral boundary - should fail
        assert_eq!(
            view.write_range(0x41E, &[0xFF; 4]), // Crosses UART boundary
            Err(ShadowError::Denied)
        );
    });
}

fn example_layered_security() {
    let storage = ShadowStorageBuilder::new()
        .total_size::<2048>()
        .block_size::<64>()
        .block_count::<32>()
        .access_policy(LayeredPolicy {
            bootloader: BootloaderProtection,
            peripheral: PeripheralAccessPolicy,
        })
        .no_persist()
        .build();
    
    let host = storage.host_shadow();
    
    host.with_view(|view| {
        // Can't write to bootloader even if it was a peripheral
        assert_eq!(
            view.write_range(0x00, &[0xFF; 4]),
            Err(ShadowError::Denied)
        );
        
        // Can't access non-peripheral memory even outside bootloader
        assert_eq!(
            view.write_range(0x200, &[0xFF; 4]),
            Err(ShadowError::Denied)
        );
        
        // Can only access allowed peripheral regions outside bootloader
        assert!(view.write_range(0x400, &[0x12; 4]).is_ok()); // UART - OK
        
        // Special case: write-only timer clear register
        assert!(view.write_range(0x608, &[0x00; 4]).is_ok());
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_access_policy_example() {
        super::main();
    }
}