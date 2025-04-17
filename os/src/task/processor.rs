//! [`Processor`] 的实现与控制流交汇。
//!
//! 维护用户应用在 CPU 上的持续运行，记录 CPU 当前运行状态，
//! 并执行不同应用间的控制流切换与转移。

use super::__switch;
use super::{fetch_task, TaskStatus};
use super::{TaskContext, TaskControlBlock};
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::sync::Arc;
use lazy_static::*;

/// 处理器管理结构体。
pub struct Processor {
    /// 当前处理器上正在执行的任务。
    current: Option<Arc<TaskControlBlock>>,

    /// 每个核的基本控制流，辅助选择和切换进程。
    idle_task_cx: TaskContext,
}

impl Default for Processor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor {
    /// 创建一个空的 Processor。
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }

    /// 获取 `idle_task_cx` 的可变指针。
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }

    /// 获取当前任务（移动语义）。
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }

    /// 获取当前任务（克隆语义）。
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(Arc::clone)
    }
}

lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}

/// 进程执行与调度的主要部分。
/// 循环调用 `fetch_task` 获取需要运行的进程，并通过 `__switch` 切换进程。
pub fn run_tasks() {
    loop {
        let mut processor = PROCESSOR.exclusive_access();
        if let Some(task) = fetch_task() {
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
            // 独占访问即将运行的任务 TCB
            let mut task_inner = task.inner_exclusive_access();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            task_inner.task_status = TaskStatus::Running;
            // 手动释放 task_inner
            drop(task_inner);
            // 手动释放 task TCB
            processor.current = Some(task);
            // 手动释放 processor
            drop(processor);
            unsafe {
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
            }
        } else {
            warn!("no tasks available in run_tasks");
        }
    }
}

/// 通过 take 获取当前任务，并置为 None。
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}

/// 获取当前任务的克隆。
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

/// 获取当前用户 token（页表地址）。
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    task.get_user_token()
}

/// 获取当前任务的 trap context 可变引用。
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_cx()
}

/// 返回到 idle 控制流以进行新一轮调度。
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}
