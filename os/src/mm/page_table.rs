//! [`PageTableEntry`] 和 [`PageTable`] 的实现。

use super::{frame_alloc, FrameTracker, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
use crate::utils::do_nothing;
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;

bitflags! {
    /// 页表项标志位
    pub struct PTEFlags: u8 {
        /// 有效位
        const V = 1 << 0;
        /// 可读
        const R = 1 << 1;
        /// 可写
        const W = 1 << 2;
        /// 可执行
        const X = 1 << 3;
        /// 用户
        const U = 1 << 4;
        /// 全局
        const G = 1 << 5;
        /// 已访问
        const A = 1 << 6;
        /// 已修改
        const D = 1 << 7;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
/// 页表项结构

pub struct PageTableEntry {
    /// 页表项的位
    pub bits: usize,
}

impl PageTableEntry {
    /// 创建一个新的页表项

    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {

        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }

    /// 创建一个空的页表项

    pub fn empty() -> Self {

        PageTableEntry { bits: 0 }
    }

    /// 获取页表项中的物理页号

    pub fn ppn(&self) -> PhysPageNum {

        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }

    /// 获取页表项中的标志位

    pub fn flags(&self) -> PTEFlags {

        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    /// 页表项所指向的页是否有效？

    pub fn is_valid(&self) -> bool {

        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }

    /// 页表项所指向的页是否可读？

    pub fn readable(&self) -> bool {

        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }

    /// 页表项所指向的页是否可写？

    pub fn writable(&self) -> bool {

        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }

    /// 页表项所指向的页是否可执行？

    pub fn executable(&self) -> bool {

        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

/// 页表结构

pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

impl Default for PageTable {
    fn default() -> Self {

        Self::new()
    }
}

/// 假设在创建/映射时不会出现内存不足的情况。

impl PageTable {
    /// 创建一个新的页表

    pub fn new() -> Self {

        let frame = frame_alloc().unwrap();

        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }

    /// 临时用于从用户空间获取参数。

    pub fn from_token(satp: usize) -> Self {

        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }

    /// 通过虚拟页号查找页表项，如果不存在，则为 4KB 页表创建一个帧

    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {

        let idxs = vpn.indexes();

        let mut ppn = self.root_ppn;

        let mut result: Option<&mut PageTableEntry> = None;

        for (i, idx) in idxs.iter().enumerate() {

            let arr = ppn.get_pte_array();

            let pte = &mut arr[*idx];

            if i == 2 {

                result = Some(pte);

                break;
            }

            if !pte.is_valid() {

                let frame = frame_alloc().unwrap();

                let phys_page_num = frame.ppn;

                let flags = PTEFlags::V;

                *pte = PageTableEntry::new(phys_page_num, flags);

                self.frames.push(frame);
            }

            let page_num = pte.ppn();

            ppn = page_num;
        }

        do_nothing();

        result
    }

    /// 通过虚拟页号查找页表项

    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {

        let idxs = vpn.indexes();

        let mut ppn = self.root_ppn;

        let mut result: Option<&mut PageTableEntry> = None;

        for (i, idx) in idxs.iter().enumerate() {

            let pte = &mut ppn.get_pte_array()[*idx];

            if i == 2 {

                result = Some(pte);

                break;
            }

            if !pte.is_valid() {

                return None;
            }

            ppn = pte.ppn();
        }

        result
    }

    /// 设置虚拟页号和物理页号之间的映射
    #[allow(unused)]

    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {

        let pte = self.find_pte_create(vpn).unwrap();

        assert!(!pte.is_valid(), "虚拟页 {:?} 在映射前已被映射", vpn);

        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);

        do_nothing();
    }

    /// 移除虚拟页号和物理页号之间的映射
    #[allow(unused)]

    pub fn unmap(&mut self, vpn: VirtPageNum) {

        let pte = self.find_pte(vpn).unwrap();

        assert!(pte.is_valid(), "虚拟页 {:?} 在取消映射前无效", vpn);

        *pte = PageTableEntry::empty();
    }

    /// 根据虚拟页号获取页表项

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {

        self.find_pte(vpn).map(|pte| *pte)
    }

    /// 获取页表的令牌

    pub fn token(&self) -> usize {

        8usize << 60 | self.root_ppn.0
    }
}

/// 通过页表将长度为 LENGTH 的 ptr[u8] 数组转换并复制到一个可变的 u8 Vec 中

pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {

    let page_table = PageTable::from_token(token);

    let mut start = ptr as usize;

    let end = start + len;

    let mut v = Vec::new();

    while start < end {

        let start_va = VirtAddr::from(start);

        let mut vpn = start_va.floor();

        let ppn = page_table.translate(vpn).unwrap().ppn();

        vpn.step();

        let mut end_va: VirtAddr = vpn.into();

        end_va = end_va.min(VirtAddr::from(end));

        if end_va.page_offset() == 0 {

            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {

            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }

        start = end_va.into();
    }

    v
}
