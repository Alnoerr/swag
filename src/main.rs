#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use core::panic::PanicInfo;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

// Keyboard scan codes for number keys
const KEY_1: u8 = 0x02;
const KEY_2: u8 = 0x03;
const KEY_3: u8 = 0x04;
const KEY_ESC: u8 = 0x01;

// === ASYNC RUNTIME ===

// Simple task structure - using function pointers to avoid trait objects
type TaskPollFn = fn(*mut u8, &mut Context<'_>) -> Poll<()>;
type TaskDropFn = fn(*mut u8);

struct Task {
    poll_fn: Option<TaskPollFn>,
    drop_fn: Option<TaskDropFn>,
    storage: [u8; 512], // Static storage for future state
}

impl Task {
    fn new() -> Self {
        Self {
            poll_fn: None,
            drop_fn: None,
            storage: [0; 512],
        }
    }
    
    // Initialize with a future by copying its state
    fn init_with<F: Future<Output = ()> + 'static>(&mut self, future: F) {
        let size = core::mem::size_of::<F>();
        if size <= self.storage.len() {
            unsafe {
                // Copy the future into our storage
                core::ptr::copy_nonoverlapping(
                    &future as *const F as *const u8,
                    self.storage.as_mut_ptr(),
                    size
                );
            }
            
            // Set up function pointers for this specific future type
            self.poll_fn = Some(|storage: *mut u8, cx: &mut Context<'_>| {
                let future_ptr = storage as *mut F;
                let future_ref = unsafe { &mut *future_ptr };
                unsafe { Pin::new_unchecked(future_ref).poll(cx) }
            });
            
            self.drop_fn = Some(|storage: *mut u8| {
                let future_ptr = storage as *mut F;
                unsafe { core::ptr::drop_in_place(future_ptr); }
            });
            
            core::mem::forget(future); // Don't drop the original
        }
    }
    
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(poll_fn) = self.poll_fn {
            poll_fn(self.storage.as_mut_ptr(), cx)
        } else {
            Poll::Ready(())
        }
    }
    
    fn is_active(&self) -> bool {
        self.poll_fn.is_some()
    }
    
    fn deactivate(&mut self) {
        if let Some(drop_fn) = self.drop_fn.take() {
            drop_fn(self.storage.as_mut_ptr());
        }
        self.poll_fn = None;
    }
}

// Simple executor that runs tasks cooperatively
struct Executor {
    tasks: [Task; 8], // Max 8 concurrent tasks - using static allocation
    current_task: usize,
}

impl Executor {
    fn new() -> Self {
        Self {
            tasks: [
                Task::new(), Task::new(), Task::new(), Task::new(),
                Task::new(), Task::new(), Task::new(), Task::new()
            ],
            current_task: 0,
        }
    }

    fn spawn<F: Future<Output = ()> + 'static>(&mut self, future: F) -> bool {
        for task in &mut self.tasks {
            if !task.is_active() {
                task.init_with(future);
                return true;
            }
        }
        false // No free slots
    }

    fn run_step(&mut self) {
        // Round-robin through tasks
        for _ in 0..self.tasks.len() {
            let task = &mut self.tasks[self.current_task];
            if task.is_active() {
                let waker = dummy_waker();
                let mut context = Context::from_waker(&waker);
                
                match task.poll(&mut context) {
                    Poll::Ready(()) => {
                        // Task completed, deactivate it
                        task.deactivate();
                    }
                    Poll::Pending => {
                        // Task is still running, continue
                    }
                }
            }
            
            self.current_task = (self.current_task + 1) % self.tasks.len();
            break; // Only run one task per step for cooperative scheduling
        }
    }
}

// Dummy waker for our simple executor
fn dummy_waker() -> Waker {
    use core::task::{RawWaker, RawWakerVTable};
    
    fn clone(_: *const ()) -> RawWaker { dummy_raw_waker() }
    fn wake(_: *const ()) {}
    fn wake_by_ref(_: *const ()) {}
    fn drop(_: *const ()) {}

    fn dummy_raw_waker() -> RawWaker {
        RawWaker::new(core::ptr::null(), &RawWakerVTable::new(clone, wake, wake_by_ref, drop))
    }

    unsafe { Waker::from_raw(dummy_raw_waker()) }
}

// === ASYNC UTILITIES ===

// Async yield - lets other tasks run
struct Yield {
    yielded: bool,
}

impl Yield {
    fn new() -> Self {
        Self { yielded: false }
    }
}

impl Future for Yield {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.yielded = true;
            Poll::Pending
        }
    }
}

async fn yield_now() {
    Yield::new().await;
}

// Async delay
struct Delay {
    remaining: u32,
}

impl Delay {
    fn new(cycles: u32) -> Self {
        Self { remaining: cycles }
    }
}

impl Future for Delay {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.remaining == 0 {
            Poll::Ready(())
        } else {
            // Decrement in chunks to avoid blocking too long
            let chunk = self.remaining.min(1000);
            self.remaining -= chunk;
            for _ in 0..chunk {
                unsafe { core::arch::asm!("nop"); }
            }
            Poll::Pending
        }
    }
}

async fn delay(cycles: u32) {
    Delay::new(cycles).await;
}

// === VGA AND INPUT ===

// Clear the screen
fn clear_screen() {
    let vga_buffer = 0xb8000 as *mut u8;
    for i in 0..80*25 {
        unsafe {
            *vga_buffer.offset(i * 2) = b' ';
            *vga_buffer.offset(i * 2 + 1) = 0x07;
        }
    }
}

// Write text at specific position
fn write_at(text: &[u8], row: usize, col: usize, color: u8) {
    let vga_buffer = 0xb8000 as *mut u8;
    let offset = (row * 80 + col) * 2;
    
    for (i, &byte) in text.iter().enumerate() {
        if offset + i * 2 < 80 * 25 * 2 {
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
        let status: u8;
        core::arch::asm!("in al, 0x64", out("al") status);
        
        if status & 0x01 != 0 {
            let scan_code: u8;
            core::arch::asm!("in al, 0x60", out("al") scan_code);
            Some(scan_code)
        } else {
            None
        }
    }
}

// === RANDOM NUMBER GENERATOR ===

static mut RNG_STATE: u32 = 12345;

fn random() -> u32 {
    unsafe {
        RNG_STATE = RNG_STATE.wrapping_mul(1103515245).wrapping_add(12345);
        RNG_STATE
    }
}

fn get_random_char() -> u8 {
    let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()SWAG";
    chars[(random() % chars.len() as u32) as usize]
}

fn get_random_color() -> u8 {
    let colors = [0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x02, 0x03, 0x05, 0x06];
    colors[(random() % colors.len() as u32) as usize]
}

// === PANIC HANDLER ===

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
    
    for _ in 0..20 {
        clear_screen();
        
        let msg = panic_messages[message_index % panic_messages.len()];
        let color = colors[color_index % colors.len()];
        write_at(msg, 2, 24, color);
        
        write_at(b"KERNEL PANIC at swag_generator():line_MAX", 10, 18, 0x0f);
        write_at(b"Stack trace: SWAG -> MORE_SWAG -> MAXIMUM_SWAG", 12, 16, 0x07);
        write_at(b"Error code: 0xSWAG (cooperative multitasking overload)", 14, 12, 0x0c);
        
        write_at(b" $$$$$$\\  $$\\      $$\\  $$$$$$\\   $$$$$$\\", 16, 20, colors[color_index % colors.len()]);
        write_at(b"$$  __$$\\ $$ | $\\  $$ |$$  __$$\\ $$  __$$\\", 17, 19, colors[(color_index + 1) % colors.len()]);
        write_at(b"\\$$$$$$\\  $$ $$ $$\\$$ |$$$$$$$$ |$$ |$$$$\\", 18, 19, colors[(color_index + 2) % colors.len()]);
        write_at(b" \\______/ \\__/     \\__|\\__|  \\__| \\______/", 19, 19, colors[(color_index + 3) % colors.len()]);
        
        write_at(b"System halted with MAXIMUM SWAG!", 22, 24, 0x08);
        
        color_index += 1;
        message_index += 1;
        
        for _ in 0..50_000_000 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
    
    clear_screen();
    write_at(b"SYSTEM SWAG OVERLOAD COMPLETE", 12, 25, 0x0c);
    write_at(b"RIP SwagOS - Too Swag 4 This World", 14, 22, 0x08);
    
    loop {}
}

// === ASYNC APPLICATIONS ===

async fn swag_generator() {
    let colors = [0x0c, 0x0a, 0x0e, 0x0b, 0x0d, 0x09];
    let mut current_line = 0;
    let mut color_index = 0;
    
    loop {
        // Check for ESC key
        if let Some(scan_code) = read_keyboard() {
            if scan_code == KEY_ESC {
                break;
            }
        }
        
        // Write SWAG at current line
        let color = colors[color_index % colors.len()];
        write_at(b"SWAG", current_line, 38, color);
        
        // Move to next line and wrap around
        current_line = (current_line + 1) % 25;
        color_index += 1;
        
        // If we've wrapped around, clear the screen
        if current_line == 0 {
            delay(5_000_000).await;
            clear_screen();
        }
        
        delay(500_000).await;
        yield_now().await;
    }
}

async fn swag_matrix() {
    let mut columns: [u8; 80] = [0; 80];
    let mut column_speeds: [u8; 80] = [1; 80];
    
    // Initialize random speeds and positions
    for i in 0..80 {
        column_speeds[i] = ((random() % 3) + 1) as u8;
        columns[i] = (random() % 25) as u8;
    }
    
    loop {
        // Check for ESC key
        if let Some(scan_code) = read_keyboard() {
            if scan_code == KEY_ESC {
                break;
            }
        }
        
        // Update each column
        for col in 0..80 {
            columns[col] = (columns[col] + column_speeds[col]) % 25;
            
            // Clear the old trail
            for trail in 0..5 {
                let clear_row = if columns[col] >= trail { 
                    columns[col] - trail 
                } else { 
                    25 + columns[col] - trail 
                };
                if clear_row < 25 {
                    write_at(b" ", clear_row as usize, col, 0x00);
                }
            }
            
            // Draw new characters
            for i in 0..8 {
                let row = if columns[col] >= i { 
                    columns[col] - i 
                } else { 
                    25 + columns[col] - i 
                };
                if row < 25 {
                    let char_byte = get_random_char();
                    let color = if i == 0 { 
                        0x0f 
                    } else if i < 3 {
                        0x0a 
                    } else {
                        0x02 
                    };
                    
                    let final_color = if random() % 20 == 0 {
                        get_random_color()
                    } else {
                        color
                    };
                    
                    write_at(&[char_byte], row as usize, col, final_color);
                }
            }
            
            // Randomly reset column
            if random() % 100 == 0 {
                columns[col] = 0;
                column_speeds[col] = ((random() % 3) + 1) as u8;
            }
        }
        
        delay(50_000).await;
        yield_now().await;
    }
}

// Background task that adds some flair
async fn background_swag_enhancer() {
    let mut counter = 0;
    loop {
        delay(2_000_000).await;
        
        // Add some random swag sparkles to corners
        if counter % 3 == 0 {
            write_at(b"*", 0, 0, get_random_color());
            write_at(b"*", 0, 79, get_random_color());
            write_at(b"*", 24, 0, get_random_color());
            write_at(b"*", 24, 79, get_random_color());
        }
        
        counter += 1;
        yield_now().await;
    }
}

fn show_menu() {
    clear_screen();
    
    let title = b"========== SwagOS v0.0.1 ==========";
    let subtitle = b"The Most Swag Operating System Ever";
    let menu_header = b"Choose your destiny:";
    let option1 = b"1) SWAG Generator";
    let option2 = b"2) Panic!!! (now with $wag)";
    let option3 = b"3) SWAG Matrix";
    let instruction = b"Press the number key... (ESC in apps to return)";
    let tech = b"Powered by: Cooperative Multitasking";
    
    write_at(title, 5, 22, 0x0e);
    write_at(subtitle, 7, 22, 0x0a);
    write_at(menu_header, 12, 30, 0x0f);
    write_at(option1, 14, 32, 0x0a);
    write_at(option2, 15, 32, 0x0c);
    write_at(option3, 16, 32, 0x0b);
    write_at(instruction, 20, 20, 0x08);
    write_at(tech, 22, 22, 0x0d);
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let mut executor = Executor::new();
    
    // Spawn the background swag enhancer
    executor.spawn(background_swag_enhancer());
    
    loop {
        show_menu();
        
        // Wait for user input
        let mut waiting_for_input = true;
        while waiting_for_input {
            if let Some(scan_code) = read_keyboard() {
                match scan_code {
                    KEY_1 => {
                        clear_screen();
                        // Run SWAG generator cooperatively with background task
                        executor.spawn(swag_generator());
                        
                        // Run executor until SWAG generator completes
                        loop {
                            executor.run_step();
                            // Check if main task (SWAG gen) is still running - exclude background task
                            let mut has_main_task = false;
                            for i in 1..executor.tasks.len() { // Skip slot 0 (background task)
                                if executor.tasks[i].is_active() {
                                    has_main_task = true;
                                    break;
                                }
                            }
                            if !has_main_task {
                                break;
                            }
                        }
                        waiting_for_input = false;
                    }
                    KEY_2 => {
                        panic!("Maximum SWAG achieved!");
                    }
                    KEY_3 => {
                        clear_screen();
                        executor.spawn(swag_matrix());
                        
                        // Run executor until matrix completes
                        loop {
                            executor.run_step();
                            let mut has_main_task = false;
                            for i in 1..executor.tasks.len() { // Skip slot 0 (background task)
                                if executor.tasks[i].is_active() {
                                    has_main_task = true;
                                    break;
                                }
                            }
                            if !has_main_task {
                                break;
                            }
                        }
                        waiting_for_input = false;
                    }
                    _ => {}
                }
            }
            
            // Keep running background tasks even while waiting for input
            executor.run_step();
        }
    }
}