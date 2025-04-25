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
    () => {
        $crate::console::print(format_args!("\n"))
    };
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

/// 使用 RGB 颜色输出内容（24 位真彩色）。
#[allow(dead_code)]

pub fn print_rgb(r: u8, g: u8, b: u8, args: fmt::Arguments) {

    print!("\u{1B}[38;2;{};{};{}m", r, g, b);

    print!("{}", args);

    print!("\u{1B}[0m"); // 重置颜色
}

/// RGB 彩色打印宏
#[macro_export]

macro_rules! print_rgb {
    (($r:expr, $g:expr, $b:expr), $fmt:literal $(, $($arg:tt)+)?) => {
        $crate::console::print_rgb($r, $g, $b, format_args!($fmt $(, $($arg)+)?))
    }
}

/// RGB 彩色打印并换行的宏
#[macro_export]

macro_rules! println_rgb {
    (($r:expr, $g:expr, $b:expr), $fmt:literal $(, $($arg:tt)+)?) => {
        $crate::console::print_rgb($r, $g, $b, format_args!(concat!($fmt, "\n") $(, $($arg)+)?))
    }
}

/// 使用 RGB 背景色输出内容（24 位真彩色）。
#[allow(dead_code)]

pub fn print_bg_rgb(r: u8, g: u8, b: u8, args: ::core::fmt::Arguments) {

    print!("\u{1B}[48;2;{};{};{}m", r, g, b);

    print!("{}", args);

    print!("\u{1B}[0m"); // 重置颜色
}

/// RGB 背景色打印宏
#[macro_export]

macro_rules! print_bg_rgb {
    (($r:expr, $g:expr, $b:expr), $fmt:literal $(, $($arg:tt)+)?) => {
        $crate::console::print_bg_rgb($r, $g, $b, format_args!($fmt $(, $($arg)+)?))
    }
}

/// RGB 背景色打印并换行的宏
#[macro_export]

macro_rules! println_bg_rgb {
    (($r:expr, $g:expr, $b:expr), $fmt:literal $(, $($arg:tt)+)?) => {
        $crate::console::print_bg_rgb($r, $g, $b, format_args!(concat!($fmt, "\n") $(, $($arg)+)?))
    }
}

#[allow(dead_code)]
/// CSS Named Color RGB 常量

pub mod named_color {

    pub const ALICEBLUE: (u8, u8, u8) = (240, 248, 255);

    pub const ANTIQUEWHITE: (u8, u8, u8) = (250, 235, 215);

    pub const AQUA: (u8, u8, u8) = (0, 255, 255);

    pub const AQUAMARINE: (u8, u8, u8) = (127, 255, 212);

    pub const AZURE: (u8, u8, u8) = (240, 255, 255);

    pub const BEIGE: (u8, u8, u8) = (245, 245, 220);

    pub const BISQUE: (u8, u8, u8) = (255, 228, 196);

    pub const BLACK: (u8, u8, u8) = (0, 0, 0);

    pub const BLANCHEDALMOND: (u8, u8, u8) = (255, 235, 205);

    pub const BLUE: (u8, u8, u8) = (0, 0, 255);

    pub const BLUEVIOLET: (u8, u8, u8) = (138, 43, 226);

    pub const BROWN: (u8, u8, u8) = (165, 42, 42);

    pub const BURLYWOOD: (u8, u8, u8) = (222, 184, 135);

    pub const CADETBLUE: (u8, u8, u8) = (95, 158, 160);

    pub const CHARTREUSE: (u8, u8, u8) = (127, 255, 0);

    pub const CHOCOLATE: (u8, u8, u8) = (210, 105, 30);

    pub const CORAL: (u8, u8, u8) = (255, 127, 80);

    pub const CORNFLOWERBLUE: (u8, u8, u8) = (100, 149, 237);

    pub const CORNSILK: (u8, u8, u8) = (255, 248, 220);

    pub const CRIMSON: (u8, u8, u8) = (220, 20, 60);

    pub const CYAN: (u8, u8, u8) = (0, 255, 255);

    pub const DARKBLUE: (u8, u8, u8) = (0, 0, 139);

    pub const DARKCYAN: (u8, u8, u8) = (0, 139, 139);

    pub const DARKGOLDENROD: (u8, u8, u8) = (184, 134, 11);

    pub const DARKGRAY: (u8, u8, u8) = (169, 169, 169);

    pub const DARKGREEN: (u8, u8, u8) = (0, 100, 0);

    pub const DARKGREY: (u8, u8, u8) = (169, 169, 169);

    pub const DARKKHAKI: (u8, u8, u8) = (189, 183, 107);

    pub const DARKMAGENTA: (u8, u8, u8) = (139, 0, 139);

    pub const DARKOLIVEGREEN: (u8, u8, u8) = (85, 139, 47);

    pub const DARKORANGE: (u8, u8, u8) = (255, 140, 0);

    pub const DARKORCHID: (u8, u8, u8) = (153, 50, 204);

    pub const DARKRED: (u8, u8, u8) = (139, 0, 0);

    pub const DARKSALMON: (u8, u8, u8) = (233, 150, 122);

    pub const DARKSEAGREEN: (u8, u8, u8) = (143, 188, 143);

    pub const DARKSLATEBLUE: (u8, u8, u8) = (72, 61, 139);

    pub const DARKSLATEGRAY: (u8, u8, u8) = (47, 79, 79);

    pub const DARKSLATEGREY: (u8, u8, u8) = (47, 79, 79);

    pub const DARKTURQUOISE: (u8, u8, u8) = (0, 206, 209);

    pub const DARKVIOLET: (u8, u8, u8) = (148, 0, 211);

    pub const DEEPPINK: (u8, u8, u8) = (255, 20, 147);

    pub const DEEPSKYBLUE: (u8, u8, u8) = (0, 191, 255);

    pub const DIMGRAY: (u8, u8, u8) = (105, 105, 105);

    pub const DIMGREY: (u8, u8, u8) = (105, 105, 105);

    pub const DODGERBLUE: (u8, u8, u8) = (30, 144, 255);

    pub const FIREBRICK: (u8, u8, u8) = (178, 34, 34);

    pub const FLORALWHITE: (u8, u8, u8) = (255, 250, 240);

    pub const FORESTGREEN: (u8, u8, u8) = (34, 139, 34);

    pub const FUCHSIA: (u8, u8, u8) = (255, 0, 255);

    pub const GAINSBORO: (u8, u8, u8) = (220, 220, 220);

    pub const GHOSTWHITE: (u8, u8, u8) = (248, 248, 255);

    pub const GOLD: (u8, u8, u8) = (255, 215, 0);

    pub const GOLDENROD: (u8, u8, u8) = (218, 165, 32);

    pub const GRAY: (u8, u8, u8) = (128, 128, 128);

    pub const GREEN: (u8, u8, u8) = (0, 128, 0);

    pub const GREENYELLOW: (u8, u8, u8) = (173, 255, 47);

    pub const GREY: (u8, u8, u8) = (128, 128, 128);

    pub const HONEYDEW: (u8, u8, u8) = (240, 255, 240);

    pub const HOTPINK: (u8, u8, u8) = (255, 105, 180);

    pub const INDIANRED: (u8, u8, u8) = (205, 92, 92);

    pub const INDIGO: (u8, u8, u8) = (75, 0, 130);

    pub const IVORY: (u8, u8, u8) = (255, 255, 240);

    pub const KHAKI: (u8, u8, u8) = (240, 230, 140);

    pub const LAVENDER: (u8, u8, u8) = (230, 230, 250);

    pub const LAVENDERBLUSH: (u8, u8, u8) = (255, 240, 245);

    pub const LAWNGREEN: (u8, u8, u8) = (124, 252, 0);

    pub const LEMONCHIFFON: (u8, u8, u8) = (255, 250, 205);

    pub const LIGHTBLUE: (u8, u8, u8) = (173, 216, 230);

    pub const LIGHTCORAL: (u8, u8, u8) = (240, 128, 128);

    pub const LIGHTCYAN: (u8, u8, u8) = (224, 255, 255);

    pub const LIGHTGOLDENRODYELLOW: (u8, u8, u8) = (250, 250, 210);

    pub const LIGHTGRAY: (u8, u8, u8) = (211, 211, 211);

    pub const LIGHTGREEN: (u8, u8, u8) = (144, 238, 144);

    pub const LIGHTGREY: (u8, u8, u8) = (211, 211, 211);

    pub const LIGHTPINK: (u8, u8, u8) = (255, 182, 193);

    pub const LIGHTSALMON: (u8, u8, u8) = (255, 160, 122);

    pub const LIGHTSEAGREEN: (u8, u8, u8) = (32, 178, 170);

    pub const LIGHTSKYBLUE: (u8, u8, u8) = (135, 206, 250);

    pub const LIGHTSLATEGRAY: (u8, u8, u8) = (119, 136, 153);

    pub const LIGHTSLATEGREY: (u8, u8, u8) = (119, 136, 153);

    pub const LIGHTSTEELBLUE: (u8, u8, u8) = (176, 196, 222);

    pub const LIGHTYELLOW: (u8, u8, u8) = (255, 255, 224);

    pub const LIME: (u8, u8, u8) = (0, 255, 0);

    pub const LIMEGREEN: (u8, u8, u8) = (50, 205, 50);

    pub const LINEN: (u8, u8, u8) = (250, 240, 230);

    pub const MAGENTA: (u8, u8, u8) = (255, 0, 255);

    pub const MAROON: (u8, u8, u8) = (128, 0, 0);

    pub const MEDIUMAQUAMARINE: (u8, u8, u8) = (102, 205, 170);

    pub const MEDIUMBLUE: (u8, u8, u8) = (0, 0, 205);

    pub const MEDIUMORCHID: (u8, u8, u8) = (186, 85, 211);

    pub const MEDIUMPURPLE: (u8, u8, u8) = (147, 112, 219);

    pub const MEDIUMSEAGREEN: (u8, u8, u8) = (60, 179, 113);

    pub const MEDIUMSLATEBLUE: (u8, u8, u8) = (123, 104, 238);

    pub const MEDIUMSPRINGGREEN: (u8, u8, u8) = (0, 250, 154);

    pub const MEDIUMTURQUOISE: (u8, u8, u8) = (72, 209, 204);

    pub const MEDIUMVIOLETRED: (u8, u8, u8) = (199, 21, 133);

    pub const MIDNIGHTBLUE: (u8, u8, u8) = (25, 25, 112);

    pub const MINTCREAM: (u8, u8, u8) = (245, 255, 250);

    pub const MISTYROSE: (u8, u8, u8) = (255, 228, 225);

    pub const MOCCASIN: (u8, u8, u8) = (255, 228, 181);

    pub const NAVAJOWHITE: (u8, u8, u8) = (255, 222, 173);

    pub const NAVY: (u8, u8, u8) = (0, 0, 128);

    pub const OLDLACE: (u8, u8, u8) = (253, 245, 230);

    pub const OLIVE: (u8, u8, u8) = (128, 128, 0);

    pub const OLIVEDRAB: (u8, u8, u8) = (107, 142, 35);

    pub const ORANGE: (u8, u8, u8) = (255, 165, 0);

    pub const ORANGERED: (u8, u8, u8) = (255, 69, 0);

    pub const ORCHID: (u8, u8, u8) = (218, 112, 214);

    pub const PALEGOLDENROD: (u8, u8, u8) = (238, 232, 170);

    pub const PALEGREEN: (u8, u8, u8) = (152, 251, 152);

    pub const PALETURQUOISE: (u8, u8, u8) = (175, 238, 238);

    pub const PALEVIOLETRED: (u8, u8, u8) = (219, 112, 147);

    pub const PAPAYAWHIP: (u8, u8, u8) = (255, 239, 213);

    pub const PEACHPUFF: (u8, u8, u8) = (255, 218, 185);

    pub const PERU: (u8, u8, u8) = (205, 133, 63);

    pub const PINK: (u8, u8, u8) = (255, 192, 203);

    pub const PLUM: (u8, u8, u8) = (221, 160, 221);

    pub const POWDERBLUE: (u8, u8, u8) = (176, 224, 230);

    pub const PURPLE: (u8, u8, u8) = (128, 0, 128);

    pub const REBECCAPURPLE: (u8, u8, u8) = (102, 51, 153);

    pub const RED: (u8, u8, u8) = (255, 0, 0);

    pub const ROSYBROWN: (u8, u8, u8) = (188, 143, 143);

    pub const ROYALBLUE: (u8, u8, u8) = (65, 105, 225);

    pub const SADDLEBROWN: (u8, u8, u8) = (139, 69, 19);

    pub const SALMON: (u8, u8, u8) = (250, 128, 114);

    pub const SANDYBROWN: (u8, u8, u8) = (244, 164, 96);

    pub const SEAGREEN: (u8, u8, u8) = (46, 139, 87);

    pub const SEASHELL: (u8, u8, u8) = (255, 245, 238);

    pub const SIENNA: (u8, u8, u8) = (160, 82, 45);

    pub const SILVER: (u8, u8, u8) = (192, 192, 192);

    pub const SKYBLUE: (u8, u8, u8) = (135, 206, 235);

    pub const SLATEBLUE: (u8, u8, u8) = (106, 90, 205);

    pub const SLATEGRAY: (u8, u8, u8) = (112, 128, 144);

    pub const SLATEGREY: (u8, u8, u8) = (112, 128, 144);

    pub const SNOW: (u8, u8, u8) = (255, 250, 250);

    pub const SPRINGGREEN: (u8, u8, u8) = (0, 255, 127);

    pub const STEELBLUE: (u8, u8, u8) = (70, 130, 180);

    pub const TAN: (u8, u8, u8) = (210, 180, 140);

    pub const TEAL: (u8, u8, u8) = (0, 128, 128);

    pub const THISTLE: (u8, u8, u8) = (216, 191, 216);

    pub const TOMATO: (u8, u8, u8) = (255, 99, 71);

    pub const TURQUOISE: (u8, u8, u8) = (64, 224, 208);

    pub const VIOLET: (u8, u8, u8) = (238, 130, 238);

    pub const WHEAT: (u8, u8, u8) = (245, 222, 179);

    pub const WHITE: (u8, u8, u8) = (255, 255, 255);

    pub const WHITESMOKE: (u8, u8, u8) = (245, 245, 245);

    pub const YELLOW: (u8, u8, u8) = (255, 255, 0);

    pub const YELLOWGREEN: (u8, u8, u8) = (154, 205, 50);
}

/// 传入 CSS Named Color RGB 常量的前景色打印宏
#[macro_export]

macro_rules! print_named_rgb {
    ($color:expr, $fmt:literal $(, $($arg:tt)+)?) => {
        $crate::console::print_rgb($color.0, $color.1, $color.2, format_args!($fmt $(, $($arg)+)?))
    }
}

/// 传入 CSS Named Color RGB 常量的背景色打印宏
#[macro_export]

macro_rules! print_bg_named_rgb {
    ($color:expr, $fmt:literal $(, $($arg:tt)+)?) => {
        $crate::console::print_bg_rgb($color.0, $color.1, $color.2, format_args!($fmt $(, $($arg)+)?))
    }
}
