//! [`PageTableEntry`] 和 [`PageTable`] 的实现。
//!
//! 本模块实现了 RISC-V SV39 分页机制下的页表结构，包括：
//! - 页表项 (PageTableEntry) 结构及其标志位
//! - 页表 (PageTable) 的创建、映射、取消映射等操作
//! - 地址翻译以及用户空间内存访问辅助函数

use super::{frame_alloc, FrameTracker, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
use crate::utils::do_nothing;
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;

bitflags! {
    /// 页表项标志位
    ///
    /// RISC-V SV39 分页模式下页表项的低 8 位标志位
    /// 对应于 RISC-V 特权级架构中定义的页表项属性
    pub struct PTEFlags: u8 {
        /// 有效位 - 表示该页表项是否有效
        const V = 1 << 0;
        /// 可读 - 表示该页是否可读取
        const R = 1 << 1;
        /// 可写 - 表示该页是否可写入
        const W = 1 << 2;
        /// 可执行 - 表示该页是否可执行
        const X = 1 << 3;
        /// 用户 - 表示用户模式是否可访问该页
        const U = 1 << 4;
        /// 全局 - 表示该页在所有地址空间中都有效
        const G = 1 << 5;
        /// 已访问 - 表示该页自上次清除以来已被访问
        const A = 1 << 6;
        /// 已修改 - 表示该页自上次清除以来已被修改
        const D = 1 << 7;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
/// 页表项结构
///
/// RISC-V SV39 分页模式下的页表项，包含 44 位物理页号和 10 位标志位
/// 格式：[53:10] PPN, [9:0] 标志位
/// 其中 [9:8] 为保留位，[7:0] 为 PTEFlags 定义的标志位

pub struct PageTableEntry {
    /// 页表项的位，包含物理页号和标志位
    pub bits: usize,
}

impl PageTableEntry {
    /// 创建一个新的页表项
    ///
    /// # 参数
    ///
    /// * `ppn` - 物理页号
    /// * `flags` - 页表项标志位
    ///
    /// # 返回值
    ///
    /// 返回一个新的页表项，物理页号存储在高位，标志位存储在低位

    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {

        // 物理页号左移 10 位，腾出空间放标志位，然后与标志位进行按位或操作
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }

    /// 创建一个空的页表项
    ///
    /// # 返回值
    ///
    /// 返回一个 bits 为 0 的页表项，表示无效的映射

    pub fn empty() -> Self {

        // 所有位都为 0，表示一个完全无效的页表项
        PageTableEntry { bits: 0 }
    }

    /// 获取页表项中的物理页号
    ///
    /// # 返回值
    ///
    /// 从页表项中提取物理页号部分 (bits[53:10])
    /// 在 SV39 模式中，有效物理页号为 44 位

    pub fn ppn(&self) -> PhysPageNum {

        // 将 bits 右移 10 位，提取 PPN 字段，然后屏蔽掉高位，只保留 44 位的 PPN
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }

    /// 获取页表项中的标志位
    ///
    /// # 返回值
    ///
    /// 从页表项中提取标志位部分 (bits[7:0])

    pub fn flags(&self) -> PTEFlags {

        // 提取低 8 位作为标志位
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    /// 判断页表项所指向的页是否有效
    ///
    /// # 返回值
    ///
    /// 如果 Valid 位被设置，返回 true，否则返回 false

    pub fn is_valid(&self) -> bool {

        // 检查有效位是否被设置
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }

    /// 判断页表项所指向的页是否可读
    ///
    /// # 返回值
    ///
    /// 如果 Read 位被设置，返回 true，否则返回 false

    pub fn readable(&self) -> bool {

        // 检查可读位是否被设置
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }

    /// 判断页表项所指向的页是否可写
    ///
    /// # 返回值
    ///
    /// 如果 Write 位被设置，返回 true，否则返回 false

    pub fn writable(&self) -> bool {

        // 检查可写位是否被设置
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }

    /// 判断页表项所指向的页是否可执行
    ///
    /// # 返回值
    ///
    /// 如果 Execute 位被设置，返回 true，否则返回 false

    pub fn executable(&self) -> bool {

        // 检查可执行位是否被设置
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

/// 页表结构
///
/// 表示一个完整的多级页表，包含根物理页号和所有分配的页表页帧
/// 在 RISC-V SV39 中实现三级页表结构

pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

impl Default for PageTable {
    fn default() -> Self {

        // 默认构造函数，直接调用 new 方法创建新页表
        Self::new()
    }
}

/// 假设在创建/映射时不会出现内存不足的情况。

impl PageTable {
    /// 创建一个新的页表
    ///
    /// 分配一个物理页作为根页表，并返回新的页表结构
    /// 该物理页初始状态全为零，表示所有条目都无效
    ///
    /// # 返回值
    ///
    /// 返回一个新的空页表实例

    pub fn new() -> Self {

        // 为根页表分配一个物理页帧
        let frame = frame_alloc().unwrap();

        // 页帧自动被 FrameTracker::new 初始化为全 0
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }

    /// 临时用于从用户空间获取参数。
    ///
    /// 根据 satp CSR 寄存器值创建一个页表实例，不分配新的页帧
    /// 主要用于临时访问其他地址空间中的内存
    ///
    /// # 参数
    ///
    /// * `satp` - satp CSR 寄存器值，包含根页表的物理页号
    ///
    /// # 返回值
    ///
    /// 返回一个使用指定根页表的页表实例，但不拥有任何物理页帧

    pub fn from_token(satp: usize) -> Self {

        // 从 satp 寄存器值中提取物理页号（低 44 位）
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(), // 空 frames，表示不拥有页表页帧所有权
        }
    }

    /// 通过虚拟页号查找页表项，如果不存在，则为 4KB 页表创建一个帧
    ///
    /// 遍历三级页表，如果中间的页表项不存在则分配新的页表页
    /// 最终返回指向最低一级页表中的页表项的可变引用
    ///
    /// # 参数
    ///
    /// * `vpn` - 要查找的虚拟页号
    ///
    /// # 返回值
    ///
    /// 如果成功找到或创建路径，返回指向最终页表项的可变引用
    /// 如果内存分配失败，会触发 panic

    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {

        // 获取三级页表索引 [一级索引, 二级索引, 三级索引]
        let idxs = vpn.indexes();

        // 从根页表开始查找
        let mut ppn = self.root_ppn;

        // 默认无结果
        let mut result: Option<&mut PageTableEntry> = None;

        // 遍历三级页表
        for (i, idx) in idxs.iter().enumerate() {

            // 获取当前级别页表的页表项数组
            let arr = ppn.get_pte_array();

            // 获取对应索引的页表项
            let pte = &mut arr[*idx];

            if i == 2 {

                // 如果已到达第三级（最低一级）页表，直接返回页表项
                result = Some(pte);

                // 跳出循环，无需继续查找
                break;
            }

            if !pte.is_valid() {

                // 如果中间页表项无效，需要分配一个新的页表页
                let frame = frame_alloc().unwrap();

                // 获取新分配的物理页号
                let phys_page_num = frame.ppn;

                // 只设置有效位，表示这是一个中间页表项
                let flags = PTEFlags::V;

                // 创建新的页表项，指向下一级页表
                *pte = PageTableEntry::new(phys_page_num, flags);

                // 保存页帧所有权
                self.frames.push(frame);
            }

            // 获取下一级页表的物理页号
            let page_num = pte.ppn();

            // 更新 ppn 为下一级页表的物理页号
            ppn = page_num;
        }

        do_nothing();

        // 返回找到或创建的页表项
        result
    }

    /// 通过虚拟页号查找页表项
    ///
    /// 遍历三级页表，如果中间任一级的页表项无效则返回 None
    /// 不会分配新的页表页
    ///
    /// # 参数
    ///
    /// * `vpn` - 要查找的虚拟页号
    ///
    /// # 返回值
    ///
    /// 如果找到完整路径，返回指向最终页表项的可变引用
    /// 如果路径中有任何无效页表项，返回 None

    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {

        // 获取三级页表索引
        let idxs = vpn.indexes();

        // 从根页表开始查找
        let mut ppn = self.root_ppn;

        // 默认无结果
        let mut result: Option<&mut PageTableEntry> = None;

        // 遍历三级页表
        for (i, idx) in idxs.iter().enumerate() {

            // 获取当前索引对应的页表项
            let pte = &mut ppn.get_pte_array()[*idx];

            if i == 2 {

                // 如果已到达第三级页表，返回该页表项
                result = Some(pte);

                // 跳出循环
                break;
            }

            if !pte.is_valid() {

                // 如果中间任一级页表项无效，表示映射不存在
                return None;
            }

            // 继续查找下一级页表
            ppn = pte.ppn();
        }

        // 返回找到的页表项
        result
    }

    /// 设置虚拟页号和物理页号之间的映射
    ///
    /// 在页表中创建从虚拟页到物理页的映射关系
    /// 如果需要，会自动创建中间的页表页
    ///
    /// # 参数
    ///
    /// * `vpn` - 要映射的虚拟页号
    /// * `ppn` - 要映射到的物理页号
    /// * `flags` - 映射的权限标志位
    ///
    /// # Panics
    ///
    /// 如果指定的虚拟页已经被映射，会触发 panic
    #[allow(unused)]

    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {

        // 查找或创建页表路径，获取最终页表项
        let pte = self.find_pte_create(vpn).unwrap();

        // 确保该虚拟页还未被映射
        assert!(!pte.is_valid(), "虚拟页 {:?} 在映射前已被映射", vpn);

        // 创建新的页表项，将虚拟页映射到物理页，并设置权限标志
        // 同时确保设置有效位
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);

        do_nothing();
    }

    /// 移除虚拟页号和物理页号之间的映射
    ///
    /// 将页表中指定虚拟页对应的页表项置为无效
    /// 注意：这不会回收中间页表页
    ///
    /// # 参数
    ///
    /// * `vpn` - 要取消映射的虚拟页号
    ///
    /// # Panics
    ///
    /// 如果指定的虚拟页未被映射，会触发 panic
    #[allow(unused)]

    pub fn unmap(&mut self, vpn: VirtPageNum) {

        // 查找页表项
        let pte = self.find_pte(vpn).unwrap();

        // 确保该虚拟页确实被映射
        assert!(pte.is_valid(), "虚拟页 {:?} 在取消映射前无效", vpn);

        // 将页表项置为空（无效）
        *pte = PageTableEntry::empty();
    }

    /// 根据虚拟页号获取页表项
    ///
    /// 查找并返回虚拟页号对应的页表项，如果不存在则返回 None
    ///
    /// # 参数
    ///
    /// * `vpn` - 要查询的虚拟页号
    ///
    /// # 返回值
    ///
    /// 如果映射存在，返回页表项的副本
    /// 如果映射不存在，返回 None

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {

        // 查找页表项，如果找到则返回其副本
        self.find_pte(vpn).map(|pte| *pte)
    }

    /// 获取页表的令牌
    ///
    /// 返回可用于设置 satp CSR 寄存器的值，启用该页表进行地址翻译
    /// 在 SV39 模式中，令牌包含模式 (8) 和根页表的物理页号
    ///
    /// # 返回值
    ///
    /// 返回可直接写入 satp 寄存器的值
    /// 高 4 位为模式 (8 表示 SV39)，低 44 位为根页表物理页号

    pub fn token(&self) -> usize {

        // 设置页表模式为 SV39 (8)，左移 60 位作为高 4 位
        // 加上根页表的物理页号作为低位
        8usize << 60 | self.root_ppn.0
    }
}

/// 通过页表将长度为 LENGTH 的 ptr[u8] 数组转换并复制到一个可变的 u8 Vec 中
///
/// 该函数用于从用户空间安全地访问数据，处理可能跨越多个物理页的情况
///
/// # 参数
///
/// * `token` - 页表令牌，通常从进程的页表获取
/// * `ptr` - 要访问的用户空间虚拟地址指针
/// * `len` - 要访问的内存长度
///
/// # 返回值
///
/// 返回一个包含多个字节切片的向量，每个切片指向物理内存中的一部分
/// 这些切片合起来表示请求的完整内存区域

pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {

    // 根据 token 创建临时页表实例
    let page_table = PageTable::from_token(token);

    // 起始虚拟地址
    let mut start = ptr as usize;

    // 结束虚拟地址（不含）
    let end = start + len;

    // 用于存储转换后的物理内存切片
    let mut v = Vec::new();

    // 循环处理，直到覆盖整个请求的内存区域
    while start < end {

        // 将起始地址转换为虚拟地址
        let start_va = VirtAddr::from(start);

        // 获取包含起始地址的虚拟页号
        let mut vpn = start_va.floor();

        // 通过页表转换获取对应的物理页号
        let ppn = page_table.translate(vpn).unwrap().ppn();

        // 移动到下一个虚拟页
        vpn.step();

        // 计算下一页的起始地址
        let mut end_va: VirtAddr = vpn.into();

        // 如果下一页起始地址超过了请求的结束地址，使用结束地址作为界限
        end_va = end_va.min(VirtAddr::from(end));

        if end_va.page_offset() == 0 {

            // 如果结束地址恰好是页边界，获取整个页剩余部分
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {

            // 否则获取部分页内容
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }

        // 更新起始地址为当前处理的结束地址
        start = end_va.into();
    }

    // 返回所有物理内存切片
    v
}
