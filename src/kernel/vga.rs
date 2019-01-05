// This file originated from Philipp Oppermann's Rust OS blog series.
// Copyright 2015 Philipp Oppermann. Please see the original license:
// https://github.com/phil-opp/blog_os/blob/master/LICENSE-MIT
// This file has been modified from its original form.

//! VGA Buffer Access
//!
//! This module provides the ability to write characters to the screen buffer.

// TODO consider moving VGA access to arch::x86 or a device driver

use core::ptr::Unique;
use core::fmt;
use spin::Mutex;

use crate::arch::x86::KERNEL_BASE;

/// The number of rows of text
pub const BUFFER_HEIGHT: usize = 25;
/// The number of columns per row of text
pub const BUFFER_WIDTH: usize = 80;
/// The address of the VGA buffer
pub const BUFFER_ADDR: usize = KERNEL_BASE + 0xb8000;

static mut BUFFER: VgaBuffer = unsafe { VgaBuffer::new() };

/// Safe wrapper around the screen buffer
pub struct VgaBuffer {
    writer: Mutex<Writer>,
}

struct Writer {
    col: usize,
    row: usize,
    color_code: ColorCode,
    buffer: Unique<Buffer>,
}

struct Buffer {
    chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

#[derive(Clone, Copy)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

/// Wrapper around a packed foreground / background pair
#[derive(Clone, Copy)]
pub struct ColorCode(u8);

/// The various foreground and background text colors
#[allow(dead_code)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

impl VgaBuffer {
    /// Creates a new wrapper around the buffer
    const unsafe fn new() -> VgaBuffer {
        VgaBuffer {
            writer: Mutex::new(Writer {
                col: 0,
                row: 0,
                color_code: ColorCode::new(Color::White, Color::Black),
                buffer: Unique::new_unchecked(BUFFER_ADDR as *mut _),
            }),
        }
    }

    /// Sets the color code to use when drawing to screen
    pub fn set_colorcode(&self, color_code: ColorCode) {
        self.writer.lock().color_code = color_code
    }

    /// Returns the current color code
    pub fn get_colorcode(&self) -> ColorCode {
        self.writer.lock().color_code
    }

    /// Clears the entire screen
    pub fn clear(&self) {
        self.writer.lock().clear();
    }
}

impl Writer {
    /// Writes bytes to buffer
    ///
    /// This grows from top down.
    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.col >= BUFFER_WIDTH {
                    self.new_line();
                }
                let (r, c) = (self.row, self.col);
                self.buffer().chars[r][c] = ScreenChar {
                    ascii_character: byte,
                    color_code: self.color_code,
                };
                self.col += 1;
            }
        }
    }

    fn buffer(&mut self) -> &mut Buffer {
        unsafe { self.buffer.as_mut() }
    }

    /// Moves all lines up one row and clears the last line
    fn new_line(&mut self) {
        const LAST_ROW: usize = BUFFER_HEIGHT - 1;

        if self.row >= LAST_ROW {
            for row in 0..LAST_ROW {
                let buffer = self.buffer();
                buffer.chars[row] = buffer.chars[row + 1]
            }
        } else {
            self.row += 1
        }
        let row = self.row; // borrowck
        self.clear_row(row);
        self.col = 0;
    }

    /// Writes '\x20' for every column in the specified row
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        self.buffer().chars[row] = [blank; BUFFER_WIDTH];
    }

    /// Clear the contents of the entire screen buffer
    fn clear(&mut self) {
        for i in 0..BUFFER_HEIGHT {
            self.clear_row(i)
        }
        self.col = 0;
        self.row = 0;
    }
}

impl fmt::Write for VgaBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut writer = self.writer.lock();
        for byte in s.bytes() {
            writer.write_byte(byte)
        }
        Ok(())
    }
}

/// Safely returns mutable access to the global VgaBuffer
///
/// This is safe because all actual contents of struct are protected
/// by a mutex. However, we cannot implement the Write trait without
/// a mutable reference to the exterior struct.
pub fn get_vgabuffer<'a>() -> &'a mut VgaBuffer {
    unsafe { &mut BUFFER }
}

/// Prints a message in red text then stops execution
pub fn print_error(fmt: fmt::Arguments) -> ! {
    use core::fmt::Write;
    use crate::arch::generic::intrinsics;
    let vgabuffer = get_vgabuffer();
    vgabuffer.set_colorcode(ColorCode::new(Color::Red, Color::Black));
    let _ = vgabuffer.write_fmt(fmt);
    intrinsics::halt();
}

impl ColorCode {
    /// Creates a new ColorCode from the specified colors
    pub const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        $crate::vga::get_vgabuffer().write_fmt(format_args!($($arg)*)).unwrap();
    });
}
