// 用于记录内存映射信息的数据结构
use super::MapPermission;
use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};

/// 表示一个内存映射区域的记录
#[derive(Clone)]
pub struct MapRecord {
    /// 起始虚拟地址
    pub start: usize,
    /// 映射长度
    pub len: usize,
    /// 映射权限
    pub permission: MapPermission,
}

impl Debug for MapRecord {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MapRecord {{ start: {:#x}, len: {:#x}, permission: {:?} }}",
            self.start, self.len, self.permission
        )
    }
}

/// 管理映射记录的结构
#[derive(Debug)]
pub struct MapRecordManager {
    /// 映射记录列表
    records: Vec<MapRecord>,
}

impl Default for MapRecordManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MapRecordManager {
    /// 创建一个新的映射记录管理器
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// 添加一个新的映射记录
    pub fn add(&mut self, start: usize, len: usize, permission: MapPermission) {
        let record = MapRecord {
            start,
            len,
            permission,
        };
        self.records.push(record);
    }

    /// 查找特定起始地址和长度的映射记录索引
    pub fn find_index(&self, start: usize, len: usize) -> Option<usize> {
        self.records
            .iter()
            .position(|record| record.start == start && record.len == len)
    }

    /// 查找包含特定地址的映射记录索引
    pub fn find_containing_index(&self, addr: usize) -> Option<usize> {
        self.records
            .iter()
            .position(|record| addr >= record.start && addr < record.start + record.len)
    }

    /// 移除指定索引的映射记录
    pub fn remove(&mut self, index: usize) -> Option<MapRecord> {
        if index < self.records.len() {
            Some(self.records.remove(index))
        } else {
            None
        }
    }

    /// 移除指定起始地址和长度的映射记录
    pub fn remove_by_range(&mut self, start: usize, len: usize) -> Option<MapRecord> {
        if let Some(index) = self.find_index(start, len) {
            Some(self.records.remove(index))
        } else {
            None
        }
    }

    /// 检查新的映射区域是否与已有区域重叠
    pub fn check_overlap(&self, start: usize, len: usize) -> bool {
        if len == 0 {
            return false;
        }
        let end = start + len;
        self.records.iter().any(|record| {
            let record_start = record.start;
            let record_end = record.start + record.len;
            // 判断区间 [record_start, record_end) 和 [start, end) 是否重叠
            record_start < end && start < record_end
        })
    }
}
