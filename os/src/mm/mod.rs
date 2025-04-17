//! 内存管理实现
//!
//! 针对 RV64 系统的 SV39 分页虚拟内存架构，
//! 以及所有与内存管理相关的内容，如帧分配器、页表、映射区域和内存集，均在此实现。
//!
//! 每个任务或进程都有一个 memory_set 用于管理其虚拟内存。
mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use address::{StepByOne, VPNRange};
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use memory_set::remap_test;
pub use memory_set::{MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::{translated_byte_buffer, translated_refmut, translated_str, PageTableEntry};
use page_table::{PTEFlags, PageTable};
/// 初始化堆分配器、帧分配器和内核空间。
pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.exclusive_access().activate();
}
