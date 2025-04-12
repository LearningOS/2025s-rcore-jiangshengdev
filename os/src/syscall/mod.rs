//! 系统调用的实现
//!
//! 所有系统调用的单一入口点，[`syscall()`]，在用户空间
//! 希望使用 `ecall` 指令执行系统调用时被调用。在这种情况下，
//! 处理器引发一个 "来自 U-模式的环境调用" 异常，这被作为
//! [`crate::trap::trap_handler`] 中的一种情况处理。
//!
//! 为了清晰起见，每个单独的系统调用都被实现为自己的函数，命名为
//! `sys_` 加上系统调用的名称。你可以在子模块中找到这样的函数，
//! 你也应该以这种方式实现系统调用。

const SYSCALL_WRITE: usize = 64;

/// exit 系统调用

const SYSCALL_EXIT: usize = 93;

/// yield 系统调用

const SYSCALL_YIELD: usize = 124;

/// gettime 系统调用

const SYSCALL_GET_TIME: usize = 169;

/// sbrk 系统调用

const SYSCALL_SBRK: usize = 214;

/// munmap 系统调用

const SYSCALL_MUNMAP: usize = 215;

/// mmap 系统调用

const SYSCALL_MMAP: usize = 222;

/// trace 系统调用

const SYSCALL_TRACE: usize = 410;

mod fs;
mod process;

use fs::*;
use process::*;

/// 处理带有 `syscall_id` 和其他参数的系统调用异常

pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {

    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut TimeVal, args[1]),
        SYSCALL_TRACE => sys_trace(args[0], args[1], args[2]),
        SYSCALL_MMAP => sys_mmap(args[0], args[1], args[2]),
        SYSCALL_MUNMAP => sys_munmap(args[0], args[1]),
        SYSCALL_SBRK => sys_sbrk(args[0] as i32),
        _ => panic!("不支持的系统调用ID: {}", syscall_id),
    }
}
