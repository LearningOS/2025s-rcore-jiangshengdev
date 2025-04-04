//! 系统调用统计模块
//!
//! 使用UPSafeCell和堆内存来保存所有任务的系统调用统计信息

use crate::sync::UPSafeCell;
use alloc::collections::BTreeMap;
use lazy_static::*;

/// 系统调用统计信息
///
/// 对于每个任务，保存其所有系统调用的调用次数
pub struct SyscallStats {
    /// 所有任务的系统调用统计信息
    /// 外层映射：任务ID -> 内层映射
    /// 内层映射：系统调用ID -> 调用次数
    stats: BTreeMap<usize, BTreeMap<usize, usize>>,
}

impl SyscallStats {
    /// 创建一个新的系统调用统计实例
    pub fn new() -> Self {
        Self {
            stats: BTreeMap::new(),
        }
    }

    /// 记录指定任务的指定系统调用的调用次数
    pub fn record_syscall(&mut self, task_id: usize, syscall_id: usize) {
        // 如果任务映射不存在，则创建一个空的映射
        let task_stats = self.stats.entry(task_id).or_default();
        // 在任务映射中，增加对应系统调用的计数
        *task_stats.entry(syscall_id).or_insert(0) += 1;
    }

    /// 获取指定任务的指定系统调用的调用次数
    pub fn get_syscall_count(&self, task_id: usize, syscall_id: usize) -> usize {
        // 获取任务的映射，如果没有则返回0
        self.stats
            .get(&task_id)
            .and_then(|task_stats| task_stats.get(&syscall_id))
            .copied()
            .unwrap_or(0)
    }
}

lazy_static! {
    /// 全局系统调用统计信息
    pub static ref SYSCALL_STATS: UPSafeCell<SyscallStats> = unsafe {
        UPSafeCell::new(SyscallStats::new())
    };
}

/// 记录指定任务的指定系统调用的调用次数
pub fn record_syscall(task_id: usize, syscall_id: usize) {
    let mut stats = SYSCALL_STATS.exclusive_access();
    stats.record_syscall(task_id, syscall_id);
}

/// 获取指定任务的指定系统调用的调用次数
pub fn get_syscall_count(task_id: usize, syscall_id: usize) -> usize {
    let stats = SYSCALL_STATS.exclusive_access();
    stats.get_syscall_count(task_id, syscall_id)
}
