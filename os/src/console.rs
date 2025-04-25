//! SBI 控制台驱动，用于文本输出。

use crate::sbi::console_putchar;
use core::fmt::{self, Write};

struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {

        for c in s.bytes() {

            console_putchar(c as usize);
        }

        Ok(())
    }
}

/// 向主机控制台输出格式化内容。
///
/// # 参数
/// * `args` - 格式化参数。

pub fn print(args: fmt::Arguments) {

    Stdout.write_fmt(args).unwrap();
}

/// 使用格式化字符串和参数向主机控制台输出 print!。
#[macro_export]

macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?))
    }
}

/// 使用格式化字符串和参数向主机控制台输出 println!。
#[macro_export]

macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?))
    }
}

/// ANSI 颜色码
#[allow(dead_code)]

pub mod color {

    // 基本前景色
    pub const BLACK: u8 = 30;

    pub const RED: u8 = 31;

    pub const GREEN: u8 = 32;

    pub const YELLOW: u8 = 33;

    pub const BLUE: u8 = 34;

    pub const MAGENTA: u8 = 35;

    pub const CYAN: u8 = 36;

    pub const WHITE: u8 = 37;

    // 亮色前景
    pub const BRIGHT_BLACK: u8 = 90;

    pub const BRIGHT_RED: u8 = 91;

    pub const BRIGHT_GREEN: u8 = 92;

    pub const BRIGHT_YELLOW: u8 = 93;

    pub const BRIGHT_BLUE: u8 = 94;

    pub const BRIGHT_MAGENTA: u8 = 95;

    pub const BRIGHT_CYAN: u8 = 96;

    pub const BRIGHT_WHITE: u8 = 97;

    // 基本背景色
    pub const BG_BLACK: u8 = 40;

    pub const BG_RED: u8 = 41;

    pub const BG_GREEN: u8 = 42;

    pub const BG_YELLOW: u8 = 43;

    pub const BG_BLUE: u8 = 44;

    pub const BG_MAGENTA: u8 = 45;

    pub const BG_CYAN: u8 = 46;

    pub const BG_WHITE: u8 = 47;

    // 亮色背景
    pub const BG_BRIGHT_BLACK: u8 = 100;

    pub const BG_BRIGHT_RED: u8 = 101;

    pub const BG_BRIGHT_GREEN: u8 = 102;

    pub const BG_BRIGHT_YELLOW: u8 = 103;

    pub const BG_BRIGHT_BLUE: u8 = 104;

    pub const BG_BRIGHT_MAGENTA: u8 = 105;

    pub const BG_BRIGHT_CYAN: u8 = 106;

    pub const BG_BRIGHT_WHITE: u8 = 107;

    // 特殊样式
    pub const RESET: u8 = 0;

    pub const BOLD: u8 = 1;

    pub const UNDERLINE: u8 = 4;

    pub const BLINK: u8 = 5;

    pub const REVERSE: u8 = 7;
}

/// 彩色输出函数
#[allow(dead_code)]

pub fn print_colorful(color_code: u8, args: fmt::Arguments) {

    print!("\u{1B}[{}m", color_code);

    print!("{}", args);

    print!("\u{1B}[0m"); // 重置颜色
}

/// 彩色打印宏，可以指定颜色码
#[macro_export]

macro_rules! print_color {
    ($color:expr, $fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print_colorful($color, format_args!($fmt $(, $($arg)+)?))
    }
}

/// 彩色打印并换行的宏
#[macro_export]

macro_rules! println_color {
    ($color:expr, $fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print_colorful($color, format_args!(concat!($fmt, "\n") $(, $($arg)+)?))
    }
}
