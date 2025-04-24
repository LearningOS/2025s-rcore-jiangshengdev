//! SBI 调用封装。

#![allow(unused)]

use core::arch::asm;

const SBI_SET_TIMER: usize = 0;

const SBI_CONSOLE_PUTCHAR: usize = 1;

const SBI_CONSOLE_GETCHAR: usize = 2;

const SBI_SHUTDOWN: usize = 8;

/// 通用 sbi 调用。
///
/// # 参数
/// * `which` - sbi 调用号。
/// * `arg0` - 第一个参数。
/// * `arg1` - 第二个参数。
/// * `arg2` - 第三个参数。
///
/// # 返回值
/// sbi 调用返回值。
#[inline(always)]

fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {

    let mut ret;

    unsafe {

        asm!(
            "ecall",
            inlateout("x10") arg0 => ret,
            in("x11") arg1,
            in("x12") arg2,
            in("x16") 0,
            in("x17") which,
        );
    }

    ret
}

/// 通过 sbi 调用设置定时器。
///
/// # 参数
/// * `timer` - 定时器时钟周期数。

pub fn set_timer(timer: usize) {

    sbi_call(SBI_SET_TIMER, timer, 0, 0);
}

/// 通过 sbi 调用在控制台输出字符（qemu uart 处理）。
///
/// # 参数
/// * `c` - 字符。

pub fn console_putchar(c: usize) {

    sbi_call(SBI_CONSOLE_PUTCHAR, c, 0, 0);
}

/// 通过 sbi 调用从控制台获取字符（qemu uart 处理）。
///
/// # 返回值
/// 获取到的字符。

pub fn console_getchar() -> usize {

    sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0)
}

/// 通过 sbi 调用关闭内核。
///
/// # 返回值
/// 不返回，直接关闭。

pub fn shutdown() -> ! {

    sbi_call(SBI_SHUTDOWN, 0, 0, 0);

    panic!("It should shutdown!");
}
