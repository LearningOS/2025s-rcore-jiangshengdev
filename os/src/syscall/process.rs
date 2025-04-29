//! Process management syscalls
use crate::task::{add_task, current_task};
use crate::{
    loader::get_app_data_by_name,
    mm::{
        parse_prot, translated_refmut, translated_str, write_user_struct, MapPermission, VirtAddr,
    },
    task::{current_user_token, exit_current_and_run_next, suspend_current_and_run_next},
    timer::get_time_us,
};
use alloc::sync::Arc;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!(
        "kernel::pid[{}] sys_waitpid [{}]",
        current_task().unwrap().pid.0,
        pid
    );
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

// 获取当前时间（秒和微秒）
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    // 打印调试信息，包含当前进程 pid
    trace!("kernel:pid[{}] sys_get_time", current_task().unwrap().pid.0);
    // 获取当前时间（微秒）
    let us = get_time_us();
    // 构造 TimeVal 结构体
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    // 获取当前用户地址空间 token
    let token = current_user_token();
    // 写入 TimeVal 到用户空间，需处理跨页和权限
    write_user_struct(token, ts, time_val);
    // 返回 0 表示成功
    0
}

// 匿名内存映射 mmap
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel:pid[{}] sys_mmap", current_task().unwrap().pid.0);
    if len == 0 {
        return 0;
    }
    let start_va = VirtAddr::from(start);
    parse_prot(prot)
        .filter(|flags| !flags.is_empty() && start_va.aligned())
        .map(MapPermission::from)
        .map(|permission| {
            let task = current_task().unwrap();
            let mut inner = task.inner_exclusive_access();
            inner.memory_set.mmap(start_va, len, permission)
        })
        .unwrap_or(-1)
}

// 取消匿名内存映射 munmap
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_munmap", current_task().unwrap().pid.0);
    if len == 0 {
        return 0;
    }
    let start_va = VirtAddr::from(start);
    Some(start_va)
        .filter(|va| va.aligned())
        .map(|va| {
            let task = current_task().unwrap();
            let mut inner = task.inner_exclusive_access();
            inner.memory_set.munmap(va, len)
        })
        .unwrap_or(-1)
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// 新建子进程，使其执行目标程序。
/// 成功返回子进程id，否则返回 -1。
/// 可能的错误：
///     无效的文件名。
pub fn sys_spawn(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_spawn", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let current_task = current_task().unwrap();
        let new_task = current_task.spawn(data);
        let new_pid = new_task.pid.0;
        add_task(new_task);
        new_pid as isize
    } else {
        -1
    }
}

/// 设置当前进程优先级为 prio
/// 参数：prio 进程优先级，要求 prio >= 2
/// 返回值：如果输入合法则返回 prio，否则返回 -1
pub fn sys_set_priority(prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority",
        current_task().unwrap().pid.0
    );

    if prio >= 2 {
        let task = current_task().unwrap();
        let mut inner = task.inner_exclusive_access();
        inner.priority = prio as u8;
        prio
    } else {
        -1
    }
}
