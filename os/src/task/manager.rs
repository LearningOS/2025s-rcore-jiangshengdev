//! [`TaskManager`] 的实现。

use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;

/// 线程安全的 `TaskControlBlock` 队列。

pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// 简单的 FIFO 调度器。

impl Default for TaskManager {
    fn default() -> Self {

        Self::new()
    }
}

impl TaskManager {
    /// 创建一个空的 TaskManager。
    ///
    /// # 返回值
    /// 新的 TaskManager。

    pub fn new() -> Self {

        Self {
            ready_queue: VecDeque::new(),
        }
    }

    /// 将进程加入就绪队列。
    ///
    /// # 参数
    /// * `task` - 要加入的任务。

    pub fn add(&mut self, task: Arc<TaskControlBlock>) {

        self.ready_queue.push_back(task);
    }

    /// 从就绪队列取出一个进程。
    ///
    /// # 返回值
    /// 取出的任务（可选）。

    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {

        self.ready_queue.pop_front()
    }
}

/// 创建并返回 TaskManager 的 UPSafeCell 实例。

fn create_task_manager() -> UPSafeCell<TaskManager> {

    unsafe {

        UPSafeCell::new(TaskManager::new())
    }
}

lazy_static! {
    /// 通过 lazy_static! 创建的 TASK_MANAGER 实例。
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> = create_task_manager();
}

/// 将进程加入就绪队列。
///
/// # 参数
/// * `task` - 要加入的任务。

pub fn add_task(task: Arc<TaskControlBlock>) {

    //trace!("kernel: TaskManager::add_task");
    let mut task_manager = TASK_MANAGER.exclusive_access();

    task_manager.add(task);
}

/// 从就绪队列取出一个进程。
///
/// # 返回值
/// 取出的任务（可选）。

pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {

    //trace!("kernel: TaskManager::fetch_task");
    let mut task_manager = TASK_MANAGER.exclusive_access();

    task_manager.fetch()
}
