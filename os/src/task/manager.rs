//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::config::BIG_STRIDE;
use crate::sync::UPSafeCell;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::*;

///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: Vec<Arc<TaskControlBlock>>,
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
            ready_queue: Vec::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        let min_idx = self
            .ready_queue
            .iter()
            .enumerate()
            .min_by_key(|(_, q)| q.inner_exclusive_access().stride)
            .map(|(idx, _)| idx);
        if let Some(idx) = min_idx {
            let tcb = self.ready_queue.remove(idx);
            {
                let mut inner = tcb.inner_exclusive_access();
                let pass = BIG_STRIDE / inner.priority;
                inner.stride += pass;
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
