#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use core::panic::PanicInfo;

// Keyboard scan codes for number keys
const KEY_1: u8 = 0x02;
const KEY_2: u8 = 0x03;
const KEY_3: u8 = 0x04;
const KEY_ESC: u8 = 0x01;

// Simple delay function
fn delay() {
    for _ in 0..10_000_000 {
        unsafe {
            core::arch::asm!("nop");
        }
    }
}

// 5 second delay for panic
fn long_delay() {
    for _ in 0..50_000_000 {
        unsafe {
            core::arch::asm!("nop");
        }
    }
}

// Clear the screen
fn clear_screen() {
    let vga_buffer = 0xb8000 as *mut u8;
    for i in 0..80*25 {
        unsafe {
            *vga_buffer.offset(i * 2) = b' ';
            *vga_buffer.offset(i * 2 + 1) = 0x07; // Light gray on black
        }
    }
}

// Write text at specific position
fn write_at(text: &[u8], row: usize, col: usize, color: u8) {
    let vga_buffer = 0xb8000 as *mut u8;
    let offset = (row * 80 + col) * 2;
    
    for (i, &byte) in text.iter().enumerate() {
        if offset + i * 2 < 80 * 25 * 2 { // Bounds check
            unsafe {
                *vga_buffer.offset((offset + i * 2) as isize) = byte;
                *vga_buffer.offset((offset + i * 2 + 1) as isize) = color;
            }
        }
    }
}

// Read from keyboard port
fn read_keyboard() -> Option<u8> {
    unsafe {
        // Check if data is available
        let status: u8;
        core::arch::asm!("in al, 0x64", out("al") status);
        
        if status & 0x01 != 0 {
            // Read the scan code
            let scan_code: u8;
            core::arch::asm!("in al, 0x60", out("al") scan_code);
            Some(scan_code)
        } else {
            None
        }
    }
}

/// This function is called on panic - NOW WITH MAXIMUM SWAG!
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    clear_screen();
    
    let panic_messages = [
        b"OH NO! MAXIMUM SWAG OVERLOAD!!!",
        b"SYSTEM TOO SWAG TO HANDLE!!!!!!",
        b"SWAG LEVELS: OVER 9000!!!!!!!!!",
        b"ERROR: NOT ENOUGH SWAG DETECTED",
        b"PANIC: SWAG BUFFER OVERFLOW!!!!",
        b"CRITICAL: SWAG CORE MELTDOWN!!!"
    ];
    
    let colors = [0x0c, 0x0e, 0x0a, 0x0b, 0x0d, 0x09];
    let mut color_index = 0;
    let mut message_index = 0;
    
    // Animate the panic for dramatic effect
    for frame in 0..20 {
        clear_screen();
        
        // Flash different panic messages
        let msg = panic_messages[message_index % panic_messages.len()];
        let color = colors[color_index % colors.len()];
        write_at(msg, 2, 24, color);
        
        // Show some technical-looking panic info
        write_at(b"KERNEL PANIC at swag_generator():line_42", 10, 20, 0x0f);
        write_at(b"Stack trace: SWAG -> MORE_SWAG -> MAXIMUM_SWAG", 12, 16, 0x07);
        write_at(b"Error code: 0xSWAG (too much style detected)", 14, 18, 0x0c);
        
        // Show swag ASCII art
        write_at(b" $$$$$$\\  $$\\      $$\\  $$$$$$\\   $$$$$$\\", 16, 20, colors[color_index % colors.len()]);
        write_at(b"$$  __$$\\ $$ | $\\  $$ |$$  __$$\\ $$  __$$\\", 17, 19, colors[(color_index + 1) % colors.len()]);
        write_at(b"\\$$$$$$\\  $$ $$ $$\\$$ |$$$$$$$$ |$$ |$$$$\\", 18, 19, colors[(color_index + 2) % colors.len()]);
        write_at(b" \\______/ \\__/     \\__|\\__|  \\__| \\______/", 19, 19, colors[(color_index + 3) % colors.len()]);
        
        write_at(b"System halted with MAXIMUM SWAG!", 22, 24, 0x08);
        
        color_index += 1;
        message_index += 1;
        
        long_delay(); // 5 second delay
    }
    
    // Final dramatic moment
    clear_screen();
    write_at(b"SYSTEM SWAG OVERLOAD COMPLETE", 12, 25, 0x0c);
    write_at(b"RIP SwagOS - Too Swag 4 This World", 14, 22, 0x08);
    
    loop {}
}

// Show the main menu
fn show_menu() {
    clear_screen();
    
    let title = b"========== SwagOS v0.0.1 ==========";
    let subtitle = b"The Most Swag Operating System Ever";
    let menu_header = b"Choose your destiny:";
    let option1 = b"1) SWAG Generator";
    let option2 = b"2) Panic!!! (now with $wag)";
    let option3 = b"3) SWAG Matrix";
    let instruction = b"Press the number key to select... (ESC to return)";
    
    write_at(title, 5, 22, 0x0e); // Yellow
    write_at(subtitle, 7, 23, 0x07); // Light gray
    write_at(menu_header, 12, 30, 0x0f); // Bright white
    write_at(option1, 14, 32, 0x0a); // Green
    write_at(option2, 15, 32, 0x0c); // Red - danger vibes
    write_at(option3, 16, 32, 0x0b); // Cyan - matrix vibes
    write_at(instruction, 20, 20, 0x08); // Dark gray
}

// Simple random number generator (Linear Congruential Generator)
static mut RNG_STATE: u32 = 12345;

fn random() -> u32 {
    unsafe {
        RNG_STATE = RNG_STATE.wrapping_mul(1103515245).wrapping_add(12345);
        RNG_STATE
    }
}

// Get random character for matrix
fn get_random_char() -> u8 {
    let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()SWAG";
    chars[(random() % chars.len() as u32) as usize]
}

// Get random color
fn get_random_color() -> u8 {
    let colors = [0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x02, 0x03, 0x05, 0x06];
    colors[(random() % colors.len() as u32) as usize]
}

// SWAG Matrix - digital rain with style!
fn swag_matrix() {
    clear_screen();
    
    // Matrix state - tracks each column's position and speed
    let mut columns: [u8; 80] = [0; 80]; // Current row for each column
    let mut column_speeds: [u8; 80] = [1; 80]; // Speed of each column
    let mut delay_counter = 0;
    const MATRIX_DELAY: u32 = 100_000; // Faster than SWAG generator for smooth animation
    
    // Initialize random speeds for each column
    for i in 0..80 {
        column_speeds[i] = ((random() % 3) + 1) as u8; // Speed 1-3
        columns[i] = (random() % 25) as u8; // Random starting positions
    }
    
    loop {
        // Check for ESC key - async input!
        if let Some(scan_code) = read_keyboard() {
            if scan_code == KEY_ESC {
                return;
            }
        }
        
        delay_counter += 1;
        
        // Update matrix animation
        if delay_counter >= MATRIX_DELAY {
            delay_counter = 0;
            
            // Update each column
            for col in 0..80 {
                // Move column down by its speed
                columns[col] = (columns[col] + column_speeds[col]) % 25;
                
                // Clear the old trail (fade effect)
                for trail in 0..5 {
                    let clear_row = if columns[col] >= trail { columns[col] - trail } else { 25 + columns[col] - trail };
                    if clear_row < 25 {
                        write_at(b" ", clear_row as usize, col, 0x00);
                    }
                }
                
                // Draw new characters in the column
                for i in 0..8 {
                    let row = if columns[col] >= i { columns[col] - i } else { 25 + columns[col] - i };
                    if row < 25 {
                        let char_byte = get_random_char();
                        let color = if i == 0 { 
                            0x0f // Bright white at the head
                        } else if i < 3 {
                            0x0a // Bright green
                        } else {
                            0x02 // Dark green for tail
                        };
                        
                        // Sometimes add SWAG colors for extra style
                        let final_color = if random() % 20 == 0 {
                            get_random_color()
                        } else {
                            color
                        };
                        
                        write_at(&[char_byte], row as usize, col, final_color);
                    }
                }
                
                // Randomly reset column position and speed
                if random() % 100 == 0 {
                    columns[col] = 0;
                    column_speeds[col] = ((random() % 3) + 1) as u8;
                }
            }
        }
        
        // Small nop to prevent CPU meltdown
        unsafe {
            core::arch::asm!("nop");
        }
    }
}
fn swag_panic() -> ! {
    panic!("Maximum SWAG achieved - system cannot handle this level of style!");
}

fn swag_generator() {
    clear_screen();
    
    let colors = [0x0c, 0x0a, 0x0e, 0x0b, 0x0d, 0x09]; // Red, Green, Yellow, Cyan, Magenta, Blue
    let mut current_line = 0;
    let mut color_index = 0;
    let mut delay_counter = 0;
    const DELAY_CYCLES: u32 = 10_000_000; // Same as original delay but we'll count it down
    
    loop {
        // Check for keyboard input on EVERY iteration - truly async!
        if let Some(scan_code) = read_keyboard() {
            if scan_code == KEY_ESC { // ESC key - instant response!
                return;
            }
        }
        
        // Non-blocking delay counter
        delay_counter += 1;
        
        // Only draw new SWAG when delay is complete
        if delay_counter >= DELAY_CYCLES {
            delay_counter = 0; // Reset counter
            
            // Write SWAG at current line
            let color = colors[color_index % colors.len()];
            write_at(b"SWAG", current_line, 38, color);
            
            // Move to next line and wrap around
            current_line = (current_line + 1) % 25;
            color_index += 1;
            
            // If we've wrapped around, clear the screen for a fresh start
            if current_line == 0 {
                // Another delay cycle before clearing
                for _ in 0..DELAY_CYCLES {
                    // Still check for ESC during clear delay!
                    if let Some(scan_code) = read_keyboard() {
                        if scan_code == KEY_ESC {
                            return;
                        }
                    }
                }
                clear_screen();
            }
        }
        
        // Small nop to prevent CPU from going crazy
        unsafe {
            core::arch::asm!("nop");
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    loop {
        show_menu();
        
        // Wait for user input
        loop {
            if let Some(scan_code) = read_keyboard() {
                match scan_code {
                    KEY_1 => {
                        swag_generator();
                        break; // Return to menu after SWAG generator
                    }
                    KEY_2 => {
                        swag_panic(); // This will trigger the panic handler!
                    }
                    KEY_3 => {
                        swag_matrix();
                        break; // Return to menu after SWAG matrix
                    }
                    _ => {} // Ignore other keys
                }
            }
        }
    }
}
