//! 陷阱处理功能
//!
//! 对于 rCore，我们有一个单一的陷阱入口点，即 `__alltraps`。在
//! [`init()`] 中初始化时，我们将 `stvec` CSR 设置为指向它。
//!
//! 所有陷阱都经过 `__alltraps`，它在 `trap.S` 中定义。汇编
//! 语言代码只做足够的工作来恢复内核空间上下文，确保 Rust 代码
//! 能安全运行，并将控制权转交给 [`trap_handler()`]。
//!
//! 然后，它根据异常的具体情况调用不同的功能。例如，定时器
//! 中断触发任务抢占，而系统调用则转到 [`syscall()`]。

mod context;

use crate::config::{TRAMPOLINE, TRAP_CONTEXT_BASE};
use crate::syscall::syscall;
use crate::task::{
    current_trap_cx, current_user_token, exit_current_and_run_next, suspend_current_and_run_next,
};
use crate::timer::set_next_trigger;
use core::arch::{asm, global_asm};
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
};

global_asm!(include_str!("trap.S"));

/// 初始化陷阱处理

pub fn init() {

    set_kernel_trap_entry();
}

fn set_kernel_trap_entry() {

    unsafe {

        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}

fn set_user_trap_entry() {

    unsafe {

        stvec::write(TRAMPOLINE, TrapMode::Direct);
    }
}

/// 在监督者模式下启用定时器中断

pub fn enable_timer_interrupt() {

    unsafe {

        sie::set_stimer();
    }
}

/// 陷阱处理程序
#[no_mangle]

pub fn trap_handler() -> ! {

    set_kernel_trap_entry();

    let cx = current_trap_cx();

    let scause = scause::read(); // 获取陷阱原因
    let stval = stval::read(); // 获取额外值
                               // trace!("into {:?}", scause.cause());
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {

            // 无论如何都跳转到下一条指令
            cx.sepc += 4;

            // 获取系统调用返回值
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {

            println!(
                "[内核] 应用程序中的页面错误，错误地址 = {:#x}，错误指令 = {:#x}，内核已终止它。",
                stval, cx.sepc
            );

            exit_current_and_run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {

            println!("[内核] 应用程序中的非法指令，内核已终止它。");

            exit_current_and_run_next();
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {

            set_next_trigger();

            suspend_current_and_run_next();
        }
        _ => {

            panic!("不支持的陷阱 {:?}，stval = {:#x}！", scause.cause(), stval);
        }
    }

    //println!("before trap_return");
    trap_return();
}

#[no_mangle]
/// 返回用户空间
/// 设置 TRAMPOLINE 页中 __restore 汇编函数的新地址，
/// 设置寄存器 a0 = trap_cx_ptr，寄存器 a1 = 用户页表的物理地址，
/// 最后，跳转到 __restore 汇编函数的新地址

pub fn trap_return() -> ! {

    set_user_trap_entry();

    let trap_cx_ptr = TRAP_CONTEXT_BASE;

    let user_satp = current_user_token();

    extern "C" {

        fn __alltraps();

        fn __restore();

    }

    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;

    // trace!("[kernel] trap_return: ..before return");
    unsafe {

        asm!(
            "fence.i",
            "jr {restore_va}",         // 跳转到 __restore 汇编函数的新地址
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,      // a0 = 陷阱上下文的虚拟地址
            in("a1") user_satp,        // a1 = 用户页表的物理地址
            options(noreturn)
        )
    }
}

#[no_mangle]
/// 处理来自内核的陷阱
/// 未实现：内核模式的陷阱/中断/异常
/// 待办：第 9 章：I/O 设备

pub fn trap_from_kernel() -> ! {

    use riscv::register::sepc;

    trace!("stval = {:#x}, sepc = {:#x}", stval::read(), sepc::read());

    panic!("来自内核的陷阱 {:?}！", scause::read().cause());
}

pub use context::TrapContext;
