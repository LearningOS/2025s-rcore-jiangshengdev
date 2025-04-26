//! Process management syscalls
use crate::task::{add_task, current_task};
use crate::{
    loader::get_app_data_by_name,
    mm::{
        parse_prot_flags, translated_refmut, translated_str, write_user_struct, MapPermission,
        VirtAddr,
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
    // 打印调试信息，包含当前进程 pid
    trace!("kernel:pid[{}] sys_mmap", current_task().unwrap().pid.0);
    // 长度为 0 直接返回成功
    if len == 0 {
        return 0;
    }
    // 检查起始地址是否页对齐
    let start_va = VirtAddr::from(start);
    if !start_va.aligned() {
        return -1;
    }
    // 检查 prot 权限是否合法
    if prot & 0x7 == 0 {
        return -1;
    }
    // 解析权限标志
    let flags = match parse_prot_flags(prot) {
        Some(f) => f,
        None => return -1,
    };
    // 转换为 MapPermission
    let permission = MapPermission::from(flags);
    // 获取当前任务
    let task = current_task().unwrap();
    // 独占访问进程内存空间
    let mut inner = task.inner_exclusive_access();
    // 执行 mmap 操作
    inner.memory_set.mmap(start_va, len, permission)
}

// 取消匿名内存映射 munmap
pub fn sys_munmap(start: usize, len: usize) -> isize {
    // 打印调试信息，包含当前进程 pid
    trace!("kernel:pid[{}] sys_munmap", current_task().unwrap().pid.0);
    // 长度为 0 直接返回成功
    if len == 0 {
        return 0;
    }
    // 检查起始地址是否页对齐
    let start_va = VirtAddr::from(start);
    if !start_va.aligned() {
        return -1;
    }
    // 获取当前任务
    let task = current_task().unwrap();
    // 独占访问进程内存空间
    let mut inner = task.inner_exclusive_access();
    // 执行 munmap 操作
    inner.memory_set.munmap(start_va, len)
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

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(_path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}
