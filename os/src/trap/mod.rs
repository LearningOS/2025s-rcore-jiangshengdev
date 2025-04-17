//! trap 处理相关功能。
//!
//! 对于 rCore，只有一个 trap 入口，即 `__alltraps`。在 [`init()`] 初始化时，将 `stvec` CSR 设置为该入口。
//!
//! 所有 trap 都经过 `__alltraps`，其定义在 `trap.S`。汇编代码仅做必要的上下文恢复，确保 Rust 代码安全运行，并将控制权转交给 [`trap_handler()`]。
//!
//! 随后根据异常类型调用不同功能。例如，定时器中断触发任务抢占，系统调用则进入 [`syscall()`]。

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

/// 初始化 trap 处理。
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

/// 使能管态定时器中断。
pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

/// trap 处理函数。
#[no_mangle]
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    let scause = scause::read();
    let stval = stval::read();
    // trace!("into {:?}", scause.cause());
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            // 无论如何跳到下一条指令
            let mut cx = current_trap_cx();
            cx.sepc += 4;
            // 获取系统调用返回值
            let result = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]);
            // sys_exec 可能改变 cx，需重新获取
            cx = current_trap_cx();
            cx.x[10] = result as usize;
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::InstructionFault)
        | Trap::Exception(Exception::InstructionPageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            println!(
                "[kernel] trap_handler:  {:?} in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.",
                scause.cause(),
                stval,
                current_trap_cx().sepc,
            );
            // 页错误退出码
            exit_current_and_run_next(-2);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, kernel killed it.");
            // 非法指令退出码
            exit_current_and_run_next(-3);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            suspend_current_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    //println!("before trap_return");
    trap_return();
}

#[no_mangle]
/// 返回用户态。
/// 设置 TRAMPOLINE 页中 __restore 汇编函数的新地址，
/// 设置寄存器 a0 = trap_cx_ptr，a1 = 用户页表物理地址，
/// 最后跳转到 __restore 汇编函数的新地址。
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
            "jr {restore_va}",         // 跳转到 __restore 汇编函数新地址
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,      // a0 = Trap Context 虚拟地址
            in("a1") user_satp,        // a1 = 用户页表物理地址
            options(noreturn)
        )
    }
}

#[no_mangle]
/// 处理来自内核的 trap。
/// 未实现：来自内核态的 trap/中断/异常。
/// Todo: 第九章 I/O 设备。
pub fn trap_from_kernel() -> ! {
    use riscv::register::sepc;
    trace!("stval = {:#x}, sepc = {:#x}", stval::read(), sepc::read());
    panic!("a trap {:?} from kernel!", scause::read().cause());
}

pub use context::TrapContext;
