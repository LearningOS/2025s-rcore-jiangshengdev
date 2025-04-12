//! 物理和虚拟地址及页号的实现。
//!
//! 本模块实现了地址转换和页表操作所需的各种地址类型，包括：
//! - 物理地址（PhysAddr）和物理页号（PhysPageNum）
//! - 虚拟地址（VirtAddr）和虚拟页号（VirtPageNum）
//! - 它们之间的相互转换
//! - 用于内存区域表示的范围类型

use super::PageTableEntry;
use crate::config::{PAGE_SIZE, PAGE_SIZE_BITS};
use core::fmt::{self, Debug, Formatter};

/// 物理地址

/// SV39 分页模式下的物理地址宽度，单位为 bit
/// 在 RISC-V 的 SV39 实现中，物理地址被限制为 56 位

const PA_WIDTH_SV39: usize = 56;

/// SV39 分页模式下的虚拟地址宽度，单位为 bit
/// 在 RISC-V 的 SV39 实现中，虚拟地址被限制为 39 位，所以命名为 SV39

const VA_WIDTH_SV39: usize = 39;

/// SV39 分页模式下的物理页号宽度，单位为 bit
/// 计算方法：物理地址宽度 - 页内偏移位数
/// = 56 - 12 = 44 位
/// 物理页号 (PPN) 用于定位物理内存中的页框，每个 PPN 对应一个 4KB 的物理页框

const PPN_WIDTH_SV39: usize = PA_WIDTH_SV39 - PAGE_SIZE_BITS;

/// SV39 分页模式下的虚拟页号宽度，单位为 bit
/// 计算方法：虚拟地址宽度 - 页内偏移位数
/// = 39 - 12 = 27 位
/// 虚拟页号 (VPN) 用于通过页表映射到对应的物理页号

const VPN_WIDTH_SV39: usize = VA_WIDTH_SV39 - PAGE_SIZE_BITS;

/// 物理地址
///
/// 表示 RISC-V 硬件中的实际物理内存地址
/// 可通过页表将虚拟地址转换为物理地址
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]

pub struct PhysAddr(pub usize);

/// 虚拟地址
///
/// 表示程序使用的逻辑地址，需要通过页表转换为物理地址
/// 在 SV39 模式中，有效位宽为 39 位
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]

pub struct VirtAddr(pub usize);

/// 物理页号
///
/// 表示物理内存中的页框编号，每个物理页的大小为 4KB
/// 由物理地址右移页内偏移位数 (12) 得到
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]

pub struct PhysPageNum(pub usize);

/// 虚拟页号
///
/// 表示虚拟地址空间中的页编号，通过页表映射到物理页号
/// 由虚拟地址右移页内偏移位数 (12) 得到
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]

pub struct VirtPageNum(pub usize);

/// 调试实现，用于格式化输出各种地址和页号类型

/// Debug 实现：格式化输出虚拟地址，以十六进制显示

impl Debug for VirtAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {

        f.write_fmt(format_args!("VA:{:#x}", self.0))
    }
}

/// Debug 实现：格式化输出虚拟页号，以十六进制显示

impl Debug for VirtPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {

        f.write_fmt(format_args!("VPN:{:#x}", self.0))
    }
}

/// Debug 实现：格式化输出物理地址，以十六进制显示

impl Debug for PhysAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {

        f.write_fmt(format_args!("PA:{:#x}", self.0))
    }
}

/// Debug 实现：格式化输出物理页号，以十六进制显示

impl Debug for PhysPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {

        f.write_fmt(format_args!("PPN:{:#x}", self.0))
    }
}

/// T: {PhysAddr, VirtAddr, PhysPageNum, VirtPageNum}
/// T -> usize: T.0
/// usize -> T: usize.into()

/// 从 usize 转换为 PhysAddr (物理地址)
/// 通过掩码限制物理地址在 PA_WIDTH_SV39 (56 位) 范围内

impl From<usize> for PhysAddr {
    fn from(v: usize) -> Self {

        // (1 << 56) - 1
        // = 0xFFFF_FFFF_FFFF_FF
        Self(v & ((1 << PA_WIDTH_SV39) - 1))
    }
}

/// 从 usize 转换为 PhysPageNum (物理页号)
/// 通过掩码限制物理页号在 PPN_WIDTH_SV39 (44 位) 范围内

impl From<usize> for PhysPageNum {
    fn from(v: usize) -> Self {

        // (1 << PPN_WIDTH_SV39) - 1
        // 1 << (56-12) - 1
        // = (1 << 44) - 1
        // = 0xFFF_FFFF_FFFF
        Self(v & ((1 << PPN_WIDTH_SV39) - 1))
    }
}

/// 从 usize 转换为 VirtAddr (虚拟地址)
/// 通过掩码限制虚拟地址在 VA_WIDTH_SV39 (39 位) 范围内

impl From<usize> for VirtAddr {
    fn from(v: usize) -> Self {

        // (1 << 39) - 1
        // = 0x7FFF_FFFF_FF
        Self(v & ((1 << VA_WIDTH_SV39) - 1))
    }
}

/// 从 usize 转换为 VirtPageNum (虚拟页号)
/// 通过掩码限制虚拟页号在 VPN_WIDTH_SV39 (27 位) 范围内

impl From<usize> for VirtPageNum {
    fn from(v: usize) -> Self {

        // 1 << (39-12) - 1
        // = (1 << 27) - 1
        // = 0x0000_0000_07FF_FFFF
        Self(v & ((1 << VPN_WIDTH_SV39) - 1))
    }
}

/// 从 PhysAddr (物理地址) 转换为 usize
/// 直接返回内部存储的无符号整数值

impl From<PhysAddr> for usize {
    fn from(v: PhysAddr) -> Self {

        v.0
    }
}

/// 从 PhysPageNum (物理页号) 转换为 usize
/// 直接返回内部存储的无符号整数值

impl From<PhysPageNum> for usize {
    fn from(v: PhysPageNum) -> Self {

        v.0
    }
}

/// 从 VirtAddr (虚拟地址) 转换为 usize
/// 对于大于等于 2^38 的地址执行符号扩展
/// 这是 RISC-V SV39 中虚拟地址的正确表示方法

impl From<VirtAddr> for usize {
    fn from(v: VirtAddr) -> Self {

        if v.0 >= (1 << (VA_WIDTH_SV39 - 1)) {

            // 1 << (39-1)
            // = 1 << 38
            // = 0x0000_0040_0000_0000
            // ~((1 << 39) - 1)
            // = ~0x0000_007F_FFFF_FFFF
            // = 0xFFFF_FF80_0000_0000
            v.0 | (!((1 << VA_WIDTH_SV39) - 1))
        } else {

            v.0
        }
    }
}

/// 从 VirtPageNum (虚拟页号) 转换为 usize
/// 直接返回内部存储的无符号整数值

impl From<VirtPageNum> for usize {
    fn from(v: VirtPageNum) -> Self {

        v.0
    }
}

/// 虚拟地址实现
/// 提供各种处理虚拟地址的方法

impl VirtAddr {
    /// 获取（向下取整的）虚拟页号
    ///
    /// 将虚拟地址除以页大小，得到包含该地址的虚拟页号
    /// 例如：地址 0x1234 位于页号 0x1

    pub fn floor(&self) -> VirtPageNum {

        VirtPageNum(self.0 / PAGE_SIZE)
    }

    /// 获取（向上取整的）虚拟页号
    ///
    /// 如果地址不是页对齐的，则返回下一个页号
    /// 用于确保覆盖到地址所在的最后一页
    /// 例如：地址 0x1234 位于页号 0x1，但 0x1FFF 向上取整到页号 0x2

    pub fn ceil(&self) -> VirtPageNum {

        VirtPageNum((self.0 - 1 + PAGE_SIZE) / PAGE_SIZE)
    }

    /// 获取虚拟地址的页内偏移
    ///
    /// 通过掩码操作获取地址在页内的偏移量（低 12 位）
    /// 例如：地址 0x1234 的页内偏移是 0x234

    pub fn page_offset(&self) -> usize {

        self.0 & (PAGE_SIZE - 1)
    }

    /// 检查虚拟地址是否按页大小对齐
    ///
    /// 当页内偏移为 0 时，地址是页对齐的
    /// 对齐的地址可以直接转换为页号

    pub fn aligned(&self) -> bool {

        self.page_offset() == 0
    }
}

/// 从 VirtAddr (虚拟地址) 转换为 VirtPageNum (虚拟页号)
/// 要求虚拟地址必须按页面大小对齐（页内偏移为 0）
/// 然后返回其对应的虚拟页号

impl From<VirtAddr> for VirtPageNum {
    fn from(v: VirtAddr) -> Self {

        assert_eq!(v.page_offset(), 0);

        v.floor()
    }
}

/// 从 VirtPageNum (虚拟页号) 转换为 VirtAddr (虚拟地址)
/// 通过左移 PAGE_SIZE_BITS (12) 位来计算对应的起始虚拟地址

impl From<VirtPageNum> for VirtAddr {
    fn from(v: VirtPageNum) -> Self {

        Self(v.0 << PAGE_SIZE_BITS)
    }
}

impl PhysAddr {
    /// 获取（向下取整的）物理页号
    ///
    /// 将物理地址除以页大小，得到包含该地址的物理页号
    /// 例如：物理地址 0x2234 位于物理页号 0x2

    pub fn floor(&self) -> PhysPageNum {

        PhysPageNum(self.0 / PAGE_SIZE)
    }

    /// 获取（向上取整的）物理页号
    ///
    /// 如果物理地址不是页对齐的，则返回下一个物理页号
    /// 用于确保覆盖到地址所在的最后一页
    /// 例如：地址 0x2234 位于页号 0x2，但 0x2FFF 向上取整到页号 0x3

    pub fn ceil(&self) -> PhysPageNum {

        PhysPageNum((self.0 - 1 + PAGE_SIZE) / PAGE_SIZE)
    }

    /// 获取物理地址的页内偏移
    ///
    /// 通过掩码操作获取地址在物理页内的偏移量（低 12 位）
    /// 例如：地址 0x2234 的页内偏移是 0x234

    pub fn page_offset(&self) -> usize {

        self.0 & (PAGE_SIZE - 1)
    }

    /// 检查物理地址是否按页大小对齐
    ///
    /// 当页内偏移为 0 时，物理地址是页对齐的
    /// 对齐的物理地址可以直接转换为物理页号

    pub fn aligned(&self) -> bool {

        self.page_offset() == 0
    }
}

/// 从 PhysAddr (物理地址) 转换为 PhysPageNum (物理页号)
/// 要求物理地址必须按页面大小对齐（页内偏移为 0）
/// 然后返回其对应的物理页号

impl From<PhysAddr> for PhysPageNum {
    fn from(v: PhysAddr) -> Self {

        assert_eq!(v.page_offset(), 0);

        v.floor()
    }
}

/// 从 PhysPageNum (物理页号) 转换为 PhysAddr (物理地址)
/// 通过左移 PAGE_SIZE_BITS (12) 位来计算对应的起始物理地址

impl From<PhysPageNum> for PhysAddr {
    fn from(v: PhysPageNum) -> Self {

        Self(v.0 << PAGE_SIZE_BITS)
    }
}

impl VirtPageNum {
    /// 获取页表项的索引
    ///
    /// 在 SV39 分页模式中，一个虚拟页号被分为三级，每级占 9 位
    /// 这三级索引用于在页表的三级结构中查找对应的页表项
    /// 返回的数组包含三个索引值 [一级索引, 二级索引, 三级索引]

    pub fn indexes(&self) -> [usize; 3] {

        let mut vpn = self.0;

        let mut idx = [0usize; 3];

        // 从高到低提取每一级的 9 位索引
        for i in (0..3).rev() {

            // 每一级索引占用 9 位 (0..511)
            idx[i] = vpn & 511;

            // 右移 9 位，处理下一级的索引
            vpn >>= 9;
        }

        idx
    }
}

impl PhysAddr {
    /// 获取物理地址的可变引用
    ///
    /// 将物理地址转换为指向类型 T 的可变引用
    ///
    /// # 安全性
    ///
    /// 这是一个不安全的操作，因为：
    /// 1. 它假设物理地址指向的内存区域有效且已初始化
    /// 2. 它绕过了 Rust 的所有权检查，返回 'static 生命周期的引用
    ///
    /// 调用者必须确保物理地址有效且对应的内存区域已正确初始化为类型 T

    pub fn get_mut<T>(&self) -> &'static mut T {

        unsafe {

            (self.0 as *mut T).as_mut().unwrap()
        }
    }
}

impl PhysPageNum {
    /// 获取页表（页表项数组）的引用
    ///
    /// 将物理页号转换为指向页表项数组的可变引用
    /// 一个页 (4KB) 可以容纳 512 个页表项 (每项 8 字节)
    ///
    /// # 安全性
    ///
    /// 这是一个不安全的操作，调用者必须确保该物理页确实包含页表

    pub fn get_pte_array(&self) -> &'static mut [PageTableEntry] {

        let pa: PhysAddr = (*self).into();

        unsafe {

            core::slice::from_raw_parts_mut(pa.0 as *mut PageTableEntry, 512)
        }
    }

    /// 获取页（字节数组）的引用
    ///
    /// 将物理页号转换为指向字节数组的可变引用
    /// 返回一个长度为 4096 (一个页大小) 的 u8 数组
    ///
    /// # 安全性
    ///
    /// 这是一个不安全的操作，调用者必须确保对该物理页的访问是有效的

    pub fn get_bytes_array(&self) -> &'static mut [u8] {

        let pa: PhysAddr = (*self).into();

        unsafe {

            core::slice::from_raw_parts_mut(pa.0 as *mut u8, 4096)
        }
    }

    /// 获取物理页内数据的可变引用
    ///
    /// 将物理页号转换为指向类型 T 的可变引用
    ///
    /// # 安全性
    ///
    /// 这是一个不安全的操作，继承自 PhysAddr::get_mut 的不安全性
    /// 调用者必须确保物理页已正确初始化为类型 T

    pub fn get_mut<T>(&self) -> &'static mut T {

        let pa: PhysAddr = (*self).into();

        pa.get_mut()
    }
}

/// 物理/虚拟页号的迭代器

/// 定义可以按步进方式迭代的类型特征
/// 用于在内存管理中遍历连续的页号

pub trait StepByOne {
    /// 步进一个元素（页号）
    ///
    /// 将当前元素向后移动一个单位

    fn step(&mut self);
}

/// 为虚拟页号实现 StepByOne 特征
/// 使 VirtPageNum 可以在连续范围内迭代

impl StepByOne for VirtPageNum {
    fn step(&mut self) {

        // 虚拟页号加 1，即移动到下一页
        self.0 += 1;
    }
}

/// 一个用于类型 T 的简单范围结构
///
/// 表示从起点到终点的连续范围，主要用于表示连续的内存页范围
/// T 必须实现 StepByOne 特征以支持迭代
#[derive(Copy, Clone)]

pub struct SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    /// 范围的起始值（包含）
    l: T,
    /// 范围的结束值（不包含）
    r: T,
}

impl<T> SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    /// 创建一个新的范围
    ///
    /// # 参数
    ///
    /// * `start` - 范围的起始值（包含）
    /// * `end` - 范围的结束值（不包含）
    ///
    /// # Panics
    ///
    /// 如果 start 大于 end，将会 panic

    pub fn new(start: T, end: T) -> Self {

        assert!(start <= end, "start {:?} > end {:?}!", start, end);

        Self { l: start, r: end }
    }

    /// 获取范围的起始值

    pub fn get_start(&self) -> T {

        self.l
    }

    /// 获取范围的结束值

    pub fn get_end(&self) -> T {

        self.r
    }
}

/// 为 SimpleRange 实现 IntoIterator 特征
/// 使 SimpleRange 可以被用在 for 循环中

impl<T> IntoIterator for SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;

    type IntoIter = SimpleRangeIterator<T>;

    fn into_iter(self) -> Self::IntoIter {

        SimpleRangeIterator::new(self.l, self.r)
    }
}

/// 简单范围结构的迭代器
///
/// 用于遍历 SimpleRange 中的所有元素

pub struct SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    /// 当前迭代到的值
    current: T,
    /// 迭代结束的值（不包含）
    end: T,
}

impl<T> SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    /// 创建一个新的范围迭代器
    ///
    /// # 参数
    ///
    /// * `l` - 迭代的起始值
    /// * `r` - 迭代的结束值（不包含）

    pub fn new(l: T, r: T) -> Self {

        Self { current: l, end: r }
    }
}

/// 实现 Iterator 特征，使 SimpleRangeIterator 成为一个标准迭代器

impl<T> Iterator for SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;

    /// 获取迭代器中的下一个元素
    ///
    /// 如果 current 等于 end，则迭代结束，返回 None
    /// 否则返回当前值，并将 current 向前步进一次

    fn next(&mut self) -> Option<Self::Item> {

        if self.current == self.end {

            None
        } else {

            let t = self.current;

            self.current.step();

            Some(t)
        }
    }
}

/// 虚拟页号的简单范围结构
///
/// 类型别名，表示一个包含连续虚拟页号的范围
/// 常用于表示一段连续的虚拟内存区域

pub type VPNRange = SimpleRange<VirtPageNum>;
