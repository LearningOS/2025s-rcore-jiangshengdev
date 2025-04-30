//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::config::BIG_STRIDE;
use crate::sync::UPSafeCell;
use alloc::collections::BinaryHeap;
use alloc::sync::Arc;
use core::cmp::Ordering;
use lazy_static::*;

///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: BinaryHeap<ReadyTask>,
}

/// 包装 TaskControlBlock 以实现按 stride 升序排列的最小堆
#[derive(Clone)]
pub struct ReadyTask(pub Arc<TaskControlBlock>);

impl PartialEq for ReadyTask {
    fn eq(&self, other: &Self) -> bool {
        self.0.inner_exclusive_access().stride == other.0.inner_exclusive_access().stride
    }
}
impl Eq for ReadyTask {}
impl PartialOrd for ReadyTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ReadyTask {
    fn cmp(&self, other: &Self) -> Ordering {
        let a = self.0.inner_exclusive_access().stride;
        let b = other.0.inner_exclusive_access().stride;
        // BinaryHeap 默认是大顶堆（最大堆），但我们需要最小的 stride。
        // 通过反转 Ordering 实现最小堆效果：stride 越小，cmp 返回 Ordering::Greater。
        // 计算环形距离 diff（支持回绕判断）
        let diff = b.wrapping_sub(a);
        if diff == 0 {
            Ordering::Equal
        } else if diff < BIG_STRIDE / 2 {
            // diff < BIG_STRIDE/2 表示从 a 到 b 的距离小于半圈，因此视作 a < b
            Ordering::Greater
        } else {
            // 否则表示 a 距离 b 超过半圈，视作 a > b
            Ordering::Less
        }
    }
}

/// A simple FIFO scheduler.
impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: BinaryHeap::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push(ReadyTask(task));
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        if let Some(ReadyTask(tcb)) = self.ready_queue.pop() {
            {
                let mut inner = tcb.inner_exclusive_access();
                let pass = BIG_STRIDE / inner.priority;
                // stride 累加，使用 wrapping_add 支持回绕
                inner.stride = inner.stride.wrapping_add(pass);
            }
            Some(tcb)
        } else {
            None
        }
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
