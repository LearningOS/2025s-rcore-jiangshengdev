//! 任务管理实现。
//!
//! 所有与任务管理相关的内容，如任务的启动与切换，都在此实现。
//!
//! 全局唯一的 [`TaskManager`] 实例 `TASK_MANAGER` 管理整个操作系统中的所有任务。
//!
//! 全局唯一的 [`Processor`] 实例 `PROCESSOR` 监控每个核上的运行任务。
//!
//! 全局唯一的 `PID_ALLOCATOR` 为用户应用分配 pid。
//!
//! 注意 `switch.S` 中的 `__switch` 汇编函数，其控制流可能与你预期不同。

mod context;
mod id;
mod manager;
mod processor;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::loader::get_app_data_by_name;
use alloc::sync::Arc;
use lazy_static::*;
pub use manager::{fetch_task, TaskManager};
use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;
pub use id::{kstack_alloc, pid_alloc, KernelStack, PidHandle};
pub use manager::add_task;
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
    Processor,
};

/// 挂起当前“运行中”任务并运行下一个任务。

pub fn suspend_current_and_run_next() {

    // 必须有一个应用正在运行。
    let task = take_current_task().unwrap();

    // ---- 独占访问当前 TCB
    let mut task_inner = task.inner_exclusive_access();

    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;

    // 状态改为 Ready
    task_inner.task_status = TaskStatus::Ready;

    drop(task_inner);

    // ---- 释放当前 PCB

    // 放回就绪队列。
    add_task(task);

    // 跳转到调度循环
    unsafe {

        schedule(task_cx_ptr);
    }
}

/// usertests 应用的 pid（make run TEST=1 时）。

pub const IDLE_PID: usize = 0;

/// 退出当前“运行中”任务并运行下一个任务。
///
/// # 参数
/// * `exit_code` - 进程退出码。

pub fn exit_current_and_run_next(exit_code: i32) {

    // 从 Processor 取出
    let task = take_current_task().unwrap();

    let pid = task.getpid();

    if pid == IDLE_PID {

        println!(
            "[kernel] Idle process exit with exit_code {} ...",
            exit_code
        );

        panic!("All applications completed!");
    }

    // **** 独占访问当前 TCB
    let mut inner = task.inner_exclusive_access();

    // 状态改为 Zombie
    inner.task_status = TaskStatus::Zombie;

    // 记录退出码
    inner.exit_code = exit_code;

    // 不移动到父进程而是挂到 initproc 下

    // ++++++ 独占访问 initproc TCB
    {

        let mut initproc_inner = INITPROC.inner_exclusive_access();

        for child in inner.children.iter() {

            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));

            let cc = child.clone();

            initproc_inner.children.push(cc);
        }
    }

    // ++++++ 释放父 PCB

    inner.children.clear();

    // 释放用户空间
    inner.memory_set.recycle_data_pages();

    drop(inner);

    // **** 释放当前 PCB
    // 手动 drop 任务以维护引用计数
    drop(task);

    // 不需要保存任务上下文
    let mut _unused = TaskContext::zero_init();

    unsafe {

        schedule(&mut _unused as *mut _);
    }
}

/// 创建并返回初始进程的 TaskControlBlock 实例

fn create_initproc() -> Arc<TaskControlBlock> {

    let data = get_app_data_by_name("ch5b_initproc").unwrap();

    let task_control_block = TaskControlBlock::new(data);

    Arc::new(task_control_block)
}

lazy_static! {
    /// 初始进程的创建。
    ///
    /// 名称 "initproc" 可更改为其他应用名如 "usertests"，
    /// 但我们有 user_shell，无需更改。
    pub static ref INITPROC: Arc<TaskControlBlock> = create_initproc();
}

/// 添加初始进程到管理器。

pub fn add_initproc() {

    let task = INITPROC.clone();

    add_task(task);
}
