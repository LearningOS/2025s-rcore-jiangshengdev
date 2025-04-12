//! 进程管理系统调用

use crate::task::{change_program_brk, exit_current_and_run_next, suspend_current_and_run_next};

#[repr(C)]
#[derive(Debug)]

pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// 任务退出并提交退出代码

pub fn sys_exit(_exit_code: i32) -> ! {

    trace!("kernel: sys_exit");

    exit_current_and_run_next();

    panic!("sys_exit 中不可达！");
}

/// 当前任务放弃资源给其他任务

pub fn sys_yield() -> isize {

    trace!("kernel: sys_yield");

    suspend_current_and_run_next();

    0
}

/// 你的任务：获取时间，包括秒和微秒
/// 提示：你可能需要通过虚拟内存管理来重新实现它。
/// 提示：如果 [`TimeVal`] 被两个页面分割怎么办？

pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {

    trace!("kernel: sys_get_time");

    -1
}

/// 待办事项：完成 sys_trace 以通过测试用例
/// 提示：你可能需要通过虚拟内存管理来重新实现它。

pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {

    trace!("kernel: sys_trace");

    -1
}

// 你的任务：实现 mmap。
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {

    trace!("kernel: sys_mmap 尚未实现！");

    -1
}

// 你的任务：实现 munmap。
pub fn sys_munmap(_start: usize, _len: usize) -> isize {

    trace!("kernel: sys_munmap 尚未实现！");

    -1
}

/// 改变数据段大小

pub fn sys_sbrk(size: i32) -> isize {

    trace!("kernel: sys_sbrk");

    if let Some(old_brk) = change_program_brk(size) {

        old_brk as isize
    } else {

        -1
    }
}
