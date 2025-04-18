//! [`FrameAllocator`] 的实现，负责管理操作系统中的所有物理页帧。
use super::{PhysAddr, PhysPageNum};
use crate::config::MEMORY_END;
use crate::sync::UPSafeCell;
use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};
use lazy_static::*;

/// 物理页帧分配与回收的追踪器。
pub struct FrameTracker {
    /// 物理页号。
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    /// 创建一个新的 FrameTracker。
    pub fn new(ppn: PhysPageNum) -> Self {
        // 页清零。
        let bytes_array = ppn.get_bytes_array();

        for i in bytes_array {
            *i = 0;
        }

        Self { ppn }
    }
}

impl Debug for FrameTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}

trait FrameAllocator {
    /// 创建分配器实例。
    fn new() -> Self;
    /// 分配一个物理页号。
    fn alloc(&mut self) -> Option<PhysPageNum>;
    /// 回收一个物理页号。
    fn dealloc(&mut self, ppn: PhysPageNum);
}

/// 栈式物理页帧分配器实现。
pub struct StackFrameAllocator {
    current: usize,
    end: usize,
    recycled: Vec<usize>,
}

impl StackFrameAllocator {
    /// 初始化分配器，设置分配区间。
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
        // trace!("last {} Physical Frames.", self.end - self.current);
    }
}

impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }

    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(ppn) = self.recycled.pop() {
            Some(ppn.into())
        } else if self.current == self.end {
            None
        } else {
            self.current += 1;
            Some((self.current - 1).into())
        }
    }

    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        // 有效性检查。
        if ppn >= self.current || self.recycled.iter().any(|&v| v == ppn) {
            panic!("Frame ppn={:#x} has not been allocated!", ppn);
        }
        // 回收。
        self.recycled.push(ppn);
    }
}

type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {
    /// 通过 lazy_static! 创建的帧分配器实例。
    pub static ref FRAME_ALLOCATOR: UPSafeCell<FrameAllocatorImpl> =
        unsafe { UPSafeCell::new(FrameAllocatorImpl::new()) };
}

/// 使用 `ekernel` 和 `MEMORY_END` 初始化帧分配器。
pub fn init_frame_allocator() {
    extern "C" {
        fn ekernel();
    }
    FRAME_ALLOCATOR.exclusive_access().init(
        PhysAddr::from(ekernel as usize).ceil(),
        PhysAddr::from(MEMORY_END).floor(),
    );
}

/// 以 FrameTracker 形式分配一个物理页帧。
pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(FrameTracker::new)
}

/// 回收指定物理页号的物理页帧。
pub fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}

#[allow(unused)]
/// 帧分配器的简单测试。
pub fn frame_allocator_test() {
    let mut v: Vec<FrameTracker> = Vec::new();

    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }

    v.clear();

    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }

    drop(v);
    println!("frame_allocator_test passed!");
}
