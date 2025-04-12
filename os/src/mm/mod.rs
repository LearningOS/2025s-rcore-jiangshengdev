//! 内存管理实现
//!
//! SV39 页表虚拟内存架构（用于 RV64 系统）相关的实现，
//! 以及所有关于内存管理的内容，如框架分配器、页表、
//! 映射区域和内存集等，都在此实现。
//!
//! 每个任务或进程都有一个 memory_set 来控制其虚拟内存。

mod address;
pub(crate) mod debug;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use address::{StepByOne, VPNRange};
pub use debug::{print_area_mapping, print_mapped_page};
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use memory_set::remap_test;
pub use memory_set::{kernel_stack_position, MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::{translated_byte_buffer, PageTableEntry};
pub use page_table::{PTEFlags, PageTable};

/// 初始化堆分配器、框架分配器和内核空间

pub fn init() {

    heap_allocator::init_heap();

    frame_allocator::init_frame_allocator();

    KERNEL_SPACE.exclusive_access().activate();
}
