//! Critical section example: Simulating ISR/main loop access patterns
//!
//! This example demonstrates:
//! - Static storage shared between contexts
//! - Main loop writing data
//! - Simulated ISR reading dirty blocks
//! - Safe access using critical sections
//! - Testing the borrow checker and interior mutability

use embedded_shadow::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

// Type alias for our specific storage configuration
type MyStorage = ShadowStorage<256, 32, 8, AllowAllPolicy, NoPersistPolicy, NoPersist, ()>;

// Simulate a static storage instance (would be in global scope in embedded)
static mut STORAGE: Option<MyStorage> = None;
static STORAGE_READY: AtomicBool = AtomicBool::new(false);

// Simulate an interrupt flag
static INTERRUPT_PENDING: AtomicBool = AtomicBool::new(false);

fn main() {
    println!("=== Critical Section Example ===\n");

    // Initialize storage (would happen in init/main in embedded)
    unsafe {
        STORAGE = Some(
            ShadowStorageBuilder::new()
                .total_size::<256>()
                .block_size::<32>()
                .block_count::<8>()
                .default_access()
                .no_persist()
                .build(),
        );
        STORAGE_READY.store(true, Ordering::Release);
    }

    // Spawn ISR simulator thread
    let isr_thread = thread::spawn(|| {
        // Wait for storage to be ready
        while !STORAGE_READY.load(Ordering::Acquire) {
            thread::yield_now();
        }

        println!("ISR simulator: Started");

        for _ in 0..20 {
            // Check if "interrupt" is pending
            if INTERRUPT_PENDING.load(Ordering::Acquire) {
                handle_interrupt();
                INTERRUPT_PENDING.store(false, Ordering::Release);
            }
            thread::sleep(Duration::from_millis(50));
        }

        println!("ISR simulator: Stopped");
    });

    // Main loop simulation
    println!("Main loop: Starting\n");

    // Get the host view for main loop operations
    let host = unsafe {
        let storage = &raw const STORAGE;
        (*storage).as_ref().unwrap().host_shadow()
    };

    for cycle in 0..5 {
        println!("Main loop: Cycle {cycle}");

        // Write some data (sensor readings, control values, etc.)
        host.with_view(|view| {
            let addr = (cycle * 32) as u16;
            let data = vec![cycle as u8 + 0x10; 16];
            println!("  Writing {} bytes to 0x{:04X}", data.len(), addr);
            view.write_range(addr, &data).unwrap();
        });

        // Trigger "interrupt"
        INTERRUPT_PENDING.store(true, Ordering::Release);

        // Simulate other main loop work
        thread::sleep(Duration::from_millis(200));
    }

    // Let ISR thread finish
    isr_thread.join().unwrap();

    println!("\nMain loop: Complete - all operations succeeded");
}

// Simulates ISR handler
fn handle_interrupt() {
    println!("\n>>> ISR: Handling interrupt");

    let kernel = unsafe {
        let storage = &raw const STORAGE;
        (*storage).as_ref().unwrap().kernel_shadow()
    };

    // In real ISR, use with_view_unchecked to avoid critical section overhead
    // ISR context already has exclusive access (interrupts disabled)
    unsafe {
        kernel.with_view_unchecked(|view| {
            let mut dirty_found = false;

            // Check each block for dirty flag
            for block in 0..8 {
                let addr = (block * 32) as u16;

                if view.is_dirty(addr, 32).unwrap() {
                    dirty_found = true;

                    // Read the dirty data
                    let mut buffer = [0u8; 32];
                    view.read_range(addr, &mut buffer).unwrap();

                    // In real system, would write to hardware registers here
                    println!(
                        "    Block {}: Dirty - first 8 bytes: {:02X?}",
                        block,
                        &buffer[0..8]
                    );
                }
            }

            if dirty_found {
                // Clear all dirty flags after processing
                view.clear_dirty();
                println!("    Cleared dirty flags");
            } else {
                println!("    No dirty blocks");
            }
        });
    }

    println!("<<< ISR: Complete\n");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_critical_section_example() {
        super::main();
    }
}
