//! 提供匿名映射区域管理的结构与实现
//!
//! 管理 mmap 和 munmap 系统调用使用的匿名映射区域

use super::{MapPermission, PageTable, VirtAddr, VirtPageNum};
use crate::mm::memory_set::{MapArea, MapType};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// 匿名映射区域管理器
/// 负责跟踪和管理进程的匿名映射内存区域
pub struct MmapAreaManager {
    /// 存储映射区域，键为起始虚拟页号
    pub mmap_areas: BTreeMap<VirtPageNum, MapArea>,
}

impl Default for MmapAreaManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MmapAreaManager {
    /// 创建一个空的映射区域管理器
    pub fn new() -> Self {
        Self {
            mmap_areas: BTreeMap::new(),
        }
    }

    /// 检查指定区间是否与现有映射重叠
    pub fn check_overlap(&self, start_vpn: VirtPageNum, end_vpn: VirtPageNum) -> bool {
        if let Some((_, area)) = self.mmap_areas.range(..=start_vpn).next_back() {
            if area.vpn_range.get_end() > start_vpn {
                return true;
            }
        }
        if let Some((next_vpn, _)) = self.mmap_areas.range(start_vpn..).next() {
            if *next_vpn < end_vpn {
                return true;
            }
        }
        false
    }

    /// 创建新的内存映射区域
    /// 将从 start 开始，长度为 len 的虚拟内存区域与物理内存映射，具有指定的权限
    pub fn mmap(
        &mut self,
        page_table: &mut PageTable,
        start: VirtAddr,
        len: usize,
        permission: MapPermission,
    ) -> isize {
        if len == 0 {
            return 0;
        }
        let end = VirtAddr::from(start.0 + len);

        // 检查该区域是否与已有映射重叠
        let start_vpn = start.floor();
        let end_vpn = end.ceil();

        if self.check_overlap(start_vpn, end_vpn) {
            return -1;
        }

        // 创建新的映射区域并添加到mmap_areas
        let mut map_area = MapArea::new(start, end, MapType::Framed, permission);
        map_area.map(page_table);
        self.mmap_areas.insert(start_vpn, map_area);
        0
    }

    /// 取消虚存的映射
    pub fn munmap(&mut self, page_table: &mut PageTable, start: VirtAddr, len: usize) -> isize {
        if len == 0 {
            return 0;
        }
        let end = VirtAddr::from(start.0 + len);
        let start_vpn = start.floor();
        let end_vpn = end.ceil();

        match self.mmap_areas.remove(&start_vpn) {
            Some(mut area) if area.vpn_range.get_end() == end_vpn => {
                area.unmap(page_table);
                0
            }
            Some(area) => {
                // 长度不匹配，将区域放回
                self.mmap_areas.insert(start_vpn, area);
                -1
            }
            None => -1,
        }
    }

    /// 获取映射区域数量
    pub fn count(&self) -> usize {
        self.mmap_areas.len()
    }

    /// 清空所有映射
    pub fn clear(&mut self, page_table: &mut PageTable) {
        // 先收集所有键，再遍历删除
        let keys: Vec<VirtPageNum> = self.mmap_areas.keys().copied().collect();
        for key in keys {
            if let Some(mut area) = self.mmap_areas.remove(&key) {
                area.unmap(page_table);
            }
        }
    }
}
