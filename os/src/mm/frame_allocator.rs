//! [`FrameAllocator`] 的实现，
//! 控制操作系统中的所有物理页帧。
//!
//! 本模块实现了物理页帧的分配与回收机制，包括：
//! - 物理页帧的追踪器 (FrameTracker)
//! - 页帧分配器特质及其堆栈式实现 (StackFrameAllocator)
//! - 全局页帧分配器实例及其初始化与使用函数

use super::{PhysAddr, PhysPageNum};
use crate::config::MEMORY_END;
use crate::console::color;
use crate::println_color;
use crate::sync::UPSafeCell;
use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};
use lazy_static::*;

/// 物理页帧分配和回收的追踪器
///
/// 表示一个已分配的物理页帧，当生命周期结束时（被 drop）会自动回收该页帧
/// 采用 RAII 模式，确保页帧资源不会泄漏

pub struct FrameTracker {
    /// 物理页号，指向被跟踪的物理页帧
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    /// 创建一个新的 FrameTracker
    ///
    /// # 参数
    ///
    /// * `ppn` - 要跟踪的物理页号
    ///
    /// # 注意
    ///
    /// 创建时会自动将对应的物理页帧内存清零

    pub fn new(ppn: PhysPageNum) -> Self {

        // 页面清零
        let bytes_array = ppn.get_bytes_array();

        for i in bytes_array {

            // 将页帧的每个字节设置为 0，确保没有旧数据残留
            *i = 0;
        }

        Self { ppn }
    }
}

impl Debug for FrameTracker {
    /// 实现 Debug 特征，以十六进制格式打印物理页号

    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {

        f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
    }
}

impl Drop for FrameTracker {
    /// 实现 Drop 特征，当 FrameTracker 被丢弃时自动回收物理页帧
    ///
    /// 这是 RAII（资源获取即初始化）模式的关键部分，确保资源不会泄露

    fn drop(&mut self) {

        // 当 FrameTracker 超出作用域时自动调用该函数，归还物理页帧
        frame_dealloc(self.ppn);
    }
}

/// 页帧分配器特质
///
/// 定义了页帧分配器应实现的基本接口：创建、分配和回收

trait FrameAllocator {
    /// 创建一个新的分配器实例

    fn new() -> Self;

    /// 分配一个物理页帧
    ///
    /// # 返回值
    ///
    /// * `Some(PhysPageNum)` - 成功分配，返回物理页号
    /// * `None` - 分配失败，没有可用页帧

    fn alloc(&mut self) -> Option<PhysPageNum>;

    /// 回收一个物理页帧
    ///
    /// # 参数
    ///
    /// * `ppn` - 要回收的物理页号

    fn dealloc(&mut self, ppn: PhysPageNum);
}

/// 页帧分配器的堆栈式实现
///
/// 使用一个连续区间和一个回收栈来管理物理页帧：
/// - 连续区间：从 current 到 end 的未分配页帧
/// - 回收栈：已回收的页帧列表
#[derive(Debug)]

pub struct StackFrameAllocator {
    /// 当前空闲内存的起始页号
    current: usize,
    /// 可用内存的结束页号（不含此页号）
    end: usize,
    /// 已回收的页号栈，用于实现页帧回收和再利用
    recycled: Vec<usize>,
}

impl StackFrameAllocator {
    /// 初始化页帧分配器
    ///
    /// # 参数
    ///
    /// * `l` - 可用内存的起始物理页号
    /// * `r` - 可用内存的结束物理页号（不含）

    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {

        // 设置起始页号
        let current = l.0;

        self.current = current;

        // 设置结束页号
        let end = r.0;

        self.end = end;

        // 输出可用页帧数量的调试信息
        trace!("剩余 {} 个物理页帧。", self.end - self.current);

        // 使用默认 Debug 实现来打印对象
        println_color!(color::CYAN, "{:#x?}", self);
    }
}

impl FrameAllocator for StackFrameAllocator {
    /// 创建一个新的堆栈式页帧分配器
    ///
    /// 初始状态下没有可用内存区域，需要之后通过 init 方法进行初始化

    fn new() -> Self {

        // 创建一个空的分配器，需要稍后初始化
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }

    /// 分配一个物理页帧
    ///
    /// 分配策略：
    /// 1. 优先从回收栈中分配（LIFO顺序）
    /// 2. 如果回收栈为空，则从当前空闲内存区域分配
    /// 3. 如果空闲内存区域已用尽，则返回 None
    ///
    /// # 返回值
    ///
    /// * `Some(PhysPageNum)` - 成功分配，返回物理页号
    /// * `None` - 分配失败，没有可用页帧

    fn alloc(&mut self) -> Option<PhysPageNum> {

        if let Some(ppn) = self.recycled.pop() {

            // 从回收栈中获取一个之前回收的页帧
            Some(ppn.into())
        } else if self.current == self.end {

            // 未被分配的区域已用完
            None
        } else {

            // 从未分配区域分配一个新页帧
            self.current += 1;

            // 返回分配前的页号
            let ppn = self.current - 1;

            let phys_page_num = ppn.into();

            Some(phys_page_num)
        }
    }

    /// 回收一个物理页帧
    ///
    /// 将页帧放入回收栈中，以便将来重新分配
    ///
    /// # 参数
    ///
    /// * `ppn` - 要回收的物理页号
    ///
    /// # Panics
    ///
    /// 如果尝试回收未分配的页帧或重复回收，将会 panic

    fn dealloc(&mut self, ppn: PhysPageNum) {

        let ppn = ppn.0;

        // 有效性检查
        if ppn >= self.current || self.recycled.iter().any(|&v| v == ppn) {

            // 页号超出已分配范围或已在回收栈中
            panic!("页帧 ppn={:#x} 未被分配！", ppn);
        }

        // 回收
        self.recycled.push(ppn);
    }
}

/// 实际使用的页帧分配器类型
///
/// 定义一个类型别名，方便在需要时更改底层实现

type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {
    /// 全局页帧分配器实例
    ///
    /// 使用 lazy_static! 和 UPSafeCell 确保全局单例安全访问
    /// 延迟初始化，只有在首次使用时才创建
    pub static ref FRAME_ALLOCATOR: UPSafeCell<FrameAllocatorImpl> = unsafe {
        UPSafeCell::new(FrameAllocatorImpl::new())
    };
}

/// 初始化全局页帧分配器
///
/// 使用内核结束地址 (ekernel) 和物理内存结束地址 (MEMORY_END) 作为可用内存区间
/// 将这个区间内的物理内存划分为页帧并交由分配器管理

pub fn init_frame_allocator() {

    extern "C" {

        /// 内核结束地址符号，由链接器提供
        fn ekernel();

    }

    let ekernel = ekernel as usize;

    let memory_end = MEMORY_END;

    let start = PhysAddr::from(ekernel);

    let end = PhysAddr::from(memory_end);

    // 向上取整确保起始地址是页对齐的
    let l = start.ceil();

    // 向下取整确保结束地址是页对齐的
    let r = end.floor();

    // 初始化全局分配器的内存区间
    let mut frame_allocator = FRAME_ALLOCATOR.exclusive_access();

    frame_allocator.init(l, r);
}

/// 分配一个物理页帧并返回其追踪器
///
/// # 返回值
///
/// * `Some(FrameTracker)` - 成功分配，返回页帧追踪器
/// * `None` - 分配失败，没有可用页帧

pub fn frame_alloc() -> Option<FrameTracker> {

    // 从全局分配器获取页号，如果成功则包装为 FrameTracker
    let mut frame_allocator = FRAME_ALLOCATOR.exclusive_access();

    let ppn = frame_allocator.alloc();

    ppn.map(FrameTracker::new)
}

/// 回收一个具有给定物理页号的物理页帧
///
/// # 参数
///
/// * `ppn` - 要回收的物理页号

pub fn frame_dealloc(ppn: PhysPageNum) {

    // 将页号归还给全局分配器
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}

#[allow(unused)]
/// 页帧分配器的简单测试
///
/// 测试流程：
/// 1. 先分配 5 个物理页帧并打印
/// 2. 清空容器，触发自动回收
/// 3. 再次分配 5 个物理页帧并打印
/// 4. 检验是否能重复使用回收的页帧

pub fn frame_allocator_test() {

    // 用于存储分配的页帧
    let mut v: Vec<FrameTracker> = Vec::new();

    for i in 0..5 {

        // 分配 5 个页帧并存储
        let frame = frame_alloc().unwrap();

        println!("{:?}", frame);

        v.push(frame);
    }

    // 清空向量，触发 Drop 特征，自动回收所有页帧
    v.clear();

    for i in 0..5 {

        // 再次分配 5 个页帧，应能复用之前回收的页帧
        let frame = frame_alloc().unwrap();

        println!("{:?}", frame);

        v.push(frame);
    }

    // 隐式回收所有页帧
    drop(v);

    println!("页帧分配器测试通过！");
}
