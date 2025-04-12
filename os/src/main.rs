//! 主模块和入口点
//!
//! 内核的各种功能以子模块的形式实现。最重要的模块有：
//!
//! - [`trap`]：处理所有从用户空间切换到内核的情况
//! - [`task`]：任务管理
//! - [`syscall`]：系统调用处理和实现
//!
//! 操作系统也从这个模块开始。内核代码从 `entry.asm` 开始执行，
//! 之后调用 [`rust_main()`] 来初始化各种功能。（详见其源代码）
//!
//! 然后我们调用 [`task::run_first_task()`]，第一次进入用户空间。

#![deny(missing_docs)]
#![deny(warnings)]
#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

extern crate alloc;

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate log;

#[macro_use]

mod console;

pub mod config;
pub mod lang_items;
mod loader;
pub mod logging;
pub mod mm;
pub mod sbi;
pub mod sync;
pub mod syscall;
pub mod task;
pub mod timer;
pub mod trap;
mod utils;

core::arch::global_asm!(include_str!("entry.asm"));

core::arch::global_asm!(include_str!("link_app.S"));

/// 清除 BSS 段

fn clear_bss() {

    extern "C" {

        fn sbss();

        fn ebss();

    }

    unsafe {

        core::ptr::write_bytes(sbss as usize as *mut u8, 0, ebss as usize - sbss as usize);
    }
}

/// 内核日志信息

fn kernel_log_info() {

    extern "C" {

        fn stext(); // 文本段起始地址
        fn etext(); // 文本段结束地址
        fn srodata(); // 只读数据段起始地址
        fn erodata(); // 只读数据段结束地址
        fn sdata(); // 数据段起始地址
        fn edata(); // 数据段结束地址
        fn sbss(); // BSS段起始地址
        fn ebss(); // BSS段结束地址
        fn boot_stack_lower_bound(); // 栈下边界
        fn boot_stack_top(); // 栈顶
    }

    logging::init();

    println!("[kernel] Hello, world!");

    trace!(
        "[kernel] .text [{:#x}, {:#x})",
        stext as usize,
        etext as usize
    );

    debug!(
        "[kernel] .rodata [{:#x}, {:#x})",
        srodata as usize, erodata as usize
    );

    info!(
        "[kernel] .data [{:#x}, {:#x})",
        sdata as usize, edata as usize
    );

    warn!(
        "[kernel] boot_stack top=bottom={:#x}, lower_bound={:#x}",
        boot_stack_top as usize, boot_stack_lower_bound as usize
    );

    error!("[kernel] .bss [{:#x}, {:#x})", sbss as usize, ebss as usize);
}

#[no_mangle]
/// 操作系统的Rust入口点

pub fn rust_main() -> ! {

    clear_bss();

    kernel_log_info();

    mm::init();

    println!("[kernel] back to world!");

    mm::remap_test();

    trap::init();

    trap::enable_timer_interrupt();

    timer::set_next_trigger();

    task::run_first_task();

    panic!("Unreachable in rust_main!");
}
