// This file originated from Philipp Oppermann's Rust OS blog series.
// Copyright 2015 Philipp Oppermann. Please see the original license:
// https://github.com/phil-opp/blog_os/blob/master/LICENSE-MIT
// This file has been modified from its original form.

use core::ptr::Unique;
use core::fmt;
use spin::Mutex;

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

pub static WRITER: Mutex<Writer> = Mutex::new(Writer {
    col: 0,
    row: 0,
    color_code: ColorCode::new(Color::White, Color::Black),
    buffer: unsafe { Unique::new(0xb8000 as *mut _) },
});

macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        $crate::vga::WRITER.lock().write_fmt(format_args!($($arg)*)).unwrap();
    });
}

pub fn clear_screen() {
    WRITER.lock().clear();
}

pub fn print_error(fmt: fmt::Arguments) {
    use core::fmt::Write;

    let mut writer = WRITER.lock();
    let old_colorcode = writer.get_colorcode();

    writer.set_colorcode(ColorCode::new(Color::Red, Color::Black));
    let _ = writer.write_fmt(fmt);
    writer.set_colorcode(old_colorcode);
}


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

pub struct Writer {
    col: usize,
    row: usize,
    color_code: ColorCode,
    buffer: Unique<Buffer>,
}

/// Writes bytes to Buffer
///
/// This grows from top down.
impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.col >= BUFFER_WIDTH {
                    self.new_line();
                }
                self.buffer().chars[self.row][self.col] = ScreenChar {
                    ascii_character: byte,
                    color_code: self.color_code,
                };
                self.col += 1;
            }
        }
    }

    fn buffer(&mut self) -> &mut Buffer {
        unsafe { self.buffer.get_mut() }
    }

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

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        self.buffer().chars[row] = [blank; BUFFER_WIDTH];
    }

    pub fn clear(&mut self) {
        for i in 0..BUFFER_HEIGHT {
            // loop to avoid blowing stack
            self.clear_row(i)
        }
        self.col = 0;
        self.row = 0;
    }

    pub fn set_colorcode(&mut self, color_code: ColorCode) {
        self.color_code = color_code
    }

    pub fn get_colorcode(&self) -> ColorCode {
        self.color_code
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte)
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct ColorCode(u8);

impl ColorCode {
    pub const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

struct Buffer {
    chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
}