//! 进程管理相关系统调用。

use alloc::sync::Arc;

use crate::{
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next,
    },
};

#[repr(C)]
#[derive(Debug)]

pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// 任务退出并提交退出码。
///
/// # 参数
/// * `exit_code` - 进程退出码。

pub fn sys_exit(exit_code: i32) -> ! {

    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);

    exit_current_and_run_next(exit_code);

    panic!("Unreachable in sys_exit!");
}

/// 当前任务让出资源给其他任务。
///
/// # 返回值
/// 总是返回 0。

pub fn sys_yield() -> isize {

    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);

    suspend_current_and_run_next();

    0
}

/// 获取当前任务的 pid。
///
/// # 返回值
/// 当前任务的 pid。

pub fn sys_getpid() -> isize {

    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);

    current_task().unwrap().pid.0 as isize
}

/// 创建子进程。
///
/// 功能：由当前进程 fork 出一个子进程。
/// 返回值：对于子进程返回 0，对于当前进程则返回子进程的 PID 。
///
/// # 返回值
/// 新建子进程的 pid。

pub fn sys_fork() -> isize {

    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);

    let current_task = current_task().unwrap();

    let new_task = current_task.fork();

    let new_pid = new_task.pid.0;

    // 修改新任务的 trap context，因为切换后会立即返回
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();

    // 子进程 fork 返回 0
    trap_cx.x[10] = 0;

    // 添加新任务到调度器
    add_task(new_task);

    new_pid as isize
}

/// 用指定路径的程序替换当前进程。
///
/// 功能：将当前进程的地址空间清空并加载一个特定的可执行文件，返回用户态后开始它的执行。
/// 参数：字符串 path 给出了要加载的可执行文件的名字；
/// 返回值：如果出错的话（如找不到名字相符的可执行文件）则返回 -1，否则不应该返回。
/// 注意：path 必须以 "\0" 结尾，否则内核将无法确定其长度
///
/// # 参数
/// * `path` - 程序路径指针。
///
/// # 返回值
/// 成功返回 0，失败返回 -1。

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

/// 等待子进程退出。
///
/// 功能：当前进程等待一个子进程变为僵尸进程，回收其全部资源并收集其返回值。
/// 参数：pid 表示要等待的子进程的进程 ID，如果为 -1 的话表示等待任意一个子进程；
/// exit_code 表示保存子进程返回值的地址，如果这个地址为 0 的话表示不必保存。
/// 返回值：如果要等待的子进程不存在则返回 -1；否则如果要等待的子进程均未结束则返回 -2；
/// 否则返回结束的子进程的进程 ID。
///
/// 如果没有 pid 匹配的子进程，返回 -1；有但未退出，返回 -2。
///
/// # 参数
/// * `pid` - 子进程 pid。
/// * `exit_code_ptr` - 退出码写入指针。
///
/// # 返回值
/// 成功返回子进程 pid，未找到返回 -1，未退出返回 -2。

pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {

    trace!(
        "kernel::pid[{}] sys_waitpid [{}]",
        current_task().unwrap().pid.0,
        pid
    );

    let task = current_task().unwrap();

    // 查找子进程

    // ---- 独占访问当前 PCB
    let mut inner = task.inner_exclusive_access();

    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {

        return -1;
        // ---- 释放当前 PCB
    }

    let pair = inner.children.iter().enumerate().find(|(_, p)| {

        // ++++ 临时独占访问子 PCB
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ 释放子 PCB
    });

    if let Some((idx, _)) = pair {

        let child = inner.children.remove(idx);

        // 确认 child 被移出后会被释放
        assert_eq!(Arc::strong_count(&child), 1);

        let found_pid = child.getpid();

        // ++++ 临时独占访问子 PCB
        let exit_code = child.inner_exclusive_access().exit_code;

        // ++++ 释放子 PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;

        found_pid as isize
    } else {

        -2
    }
    // ---- 自动释放当前 PCB
}

/// 你的任务：获取时间（秒和微秒）。
/// 提示：可结合虚拟内存管理实现。
/// 提示：如果 [`TimeVal`] 跨页怎么办？
///
/// # 参数
/// * `_ts` - 时间结构体指针。
/// * `_tz` - 时区参数。
///
/// # 返回值
/// 未实现，返回 -1。

pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {

    trace!(
        "kernel:pid[{}] sys_get_time NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );

    -1
}

/// 你的任务：实现 mmap。
///
/// # 参数
/// * `_start` - 起始地址。
/// * `_len` - 长度。
/// * `_port` - 端口。
///
/// # 返回值
/// 未实现，返回 -1。

pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {

    trace!(
        "kernel:pid[{}] sys_mmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );

    -1
}

/// 你的任务：实现 munmap。
///
/// # 参数
/// * `_start` - 起始地址。
/// * `_len` - 长度。
///
/// # 返回值
/// 未实现，返回 -1。

pub fn sys_munmap(_start: usize, _len: usize) -> isize {

    trace!(
        "kernel:pid[{}] sys_munmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );

    -1
}

/// 改变数据段大小。
///
/// # 参数
/// * `size` - 变化的字节数。
///
/// # 返回值
/// 成功返回原 brk，失败返回 -1。

pub fn sys_sbrk(size: i32) -> isize {

    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);

    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {

        old_brk as isize
    } else {

        -1
    }
}

/// 你的任务：实现 spawn。
/// 提示：fork + exec =/= spawn
///
/// # 参数
/// * `_path` - 程序路径指针。
///
/// # 返回值
/// 未实现，返回 -1。

pub fn sys_spawn(_path: *const u8) -> isize {

    trace!(
        "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );

    -1
}

/// 你的任务：设置任务优先级。
///
/// # 参数
/// * `_prio` - 优先级。
///
/// # 返回值
/// 未实现，返回 -1。

pub fn sys_set_priority(_prio: isize) -> isize {

    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );

    -1
}
