//! 主模块与入口点。
//!
//! 内核的各项功能以子模块形式实现，主要包括：
//!
//! - [`trap`]：处理所有从用户态切换到内核的情况
//! - [`task`]：任务管理
//! - [`syscall`]：系统调用处理与实现
//! - [`mm`]：基于 SV39 的地址映射
//! - [`sync`]：静态数据结构包装，便于安全访问
//!
//! 操作系统也在本模块启动。内核代码从 `entry.asm` 开始执行，随后调用 [`rust_main()`] 初始化各项功能（详见其源码）。
//!
//! 然后调用 [`task::run_tasks()`]，首次进入用户态。

#![deny(missing_docs)]
#![deny(warnings)]
#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate log;

extern crate alloc;

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

use core::arch::global_asm;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));
/// 清空 BSS 段。
fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }

    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

/// 内核日志信息。
fn kernel_log_info() {
    extern "C" {
        fn stext(); // .text 段起始地址
        fn etext(); // .text 段结束地址
        fn srodata(); // 只读数据段起始地址
        fn erodata(); // 只读数据段结束地址
        fn sdata(); // 数据段起始地址
        fn edata(); // 数据段结束地址
        fn sbss(); // BSS 段起始地址
        fn ebss(); // BSS 段结束地址
        fn boot_stack_lower_bound(); // 启动栈底
        fn boot_stack_top(); // 启动栈顶
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
/// Rust 入口点。
pub fn rust_main() -> ! {
    clear_bss();
    kernel_log_info();
    mm::init();
    mm::remap_test();
    task::add_initproc();
    println!("after initproc!");
    trap::init();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    loader::list_apps();
    task::run_tasks();
    panic!("Unreachable in rust_main!");
}
