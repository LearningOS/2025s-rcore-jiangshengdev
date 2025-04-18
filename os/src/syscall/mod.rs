//! 系统调用实现。
//!
//! 所有系统调用的唯一入口 [`syscall()`]，在用户空间通过 `ecall` 指令发起系统调用时被调用。
//! 此时处理器会产生“U 模式环境调用”异常，由 [`crate::trap::trap_handler`] 处理。
//!
//! 为了清晰，每个系统调用都实现为独立函数，命名为 `sys_` 加系统调用名。你可以在子模块中找到这些函数，也应以这种方式实现新的系统调用。

/// read 系统调用
const SYSCALL_READ: usize = 63;

/// write 系统调用
const SYSCALL_WRITE: usize = 64;

/// exit 系统调用
const SYSCALL_EXIT: usize = 93;

/// yield 系统调用
const SYSCALL_YIELD: usize = 124;

/// setpriority 系统调用
const SYSCALL_SET_PRIORITY: usize = 140;

/// gettime 系统调用
const SYSCALL_GET_TIME: usize = 169;

/// getpid 系统调用
const SYSCALL_GETPID: usize = 172;

/// sbrk 系统调用
const SYSCALL_SBRK: usize = 214;

/// munmap 系统调用
const SYSCALL_MUNMAP: usize = 215;

/// fork 系统调用
const SYSCALL_FORK: usize = 220;

/// exec 系统调用
const SYSCALL_EXEC: usize = 221;

/// mmap 系统调用
const SYSCALL_MMAP: usize = 222;

/// waitpid 系统调用
const SYSCALL_WAITPID: usize = 260;

/// spawn 系统调用
const SYSCALL_SPAWN: usize = 400;

mod fs;
mod process;

use fs::*;
use process::*;

/// 处理带有 `syscall_id` 及参数的系统调用异常。
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_READ => sys_read(args[0], args[1] as *const u8, args[2]),
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GETPID => sys_getpid(),
        SYSCALL_FORK => sys_fork(),
        SYSCALL_EXEC => sys_exec(args[0] as *const u8),
        SYSCALL_WAITPID => sys_waitpid(args[0] as isize, args[1] as *mut i32),
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut TimeVal, args[1]),
        SYSCALL_MMAP => sys_mmap(args[0], args[1], args[2]),
        SYSCALL_MUNMAP => sys_munmap(args[0], args[1]),
        SYSCALL_SBRK => sys_sbrk(args[0] as i32),
        SYSCALL_SPAWN => sys_spawn(args[0] as *const u8),
        SYSCALL_SET_PRIORITY => sys_set_priority(args[0] as isize),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
