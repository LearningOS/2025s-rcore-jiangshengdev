//! 内存管理实现
//!
//! SV39 页表虚拟内存架构（用于 RV64 系统）相关的实现，
//! 以及所有关于内存管理的内容，如框架分配器、页表、
//! 映射区域和内存集等，都在此实现。
//!
//! 每个任务或进程都有一个 memory_set 来控制其虚拟内存。
//!
//! 本模块是操作系统内核的内存管理子系统，包含以下关键部分：
//! - 地址转换和表示（address.rs）
//! - 物理页帧分配（frame_allocator.rs）
//! - 堆内存管理（heap_allocator.rs）
//! - 虚拟内存与物理内存的映射（memory_set.rs）
//! - 多级页表的实现（page_table.rs）
//! - 调试辅助功能（debug.rs）

// 子模块声明
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

/// 初始化内存管理系统
///
/// 该函数在操作系统启动时被调用，完成内存管理子系统的初始化：
/// 1. 初始化堆分配器，使内核能够使用动态内存分配
/// 2. 初始化物理页帧分配器，管理可用物理内存
/// 3. 激活内核地址空间，设置页表并启用分页机制
///
/// 调用此函数后，内核才能正常使用内存管理功能

pub fn init() {

    // 首先初始化堆分配器，使内核能够使用动态内存分配功能（如 Vec、Box 等）
    heap_allocator::init_heap();

    // 其次初始化物理页帧分配器，管理系统的物理内存资源
    frame_allocator::init_frame_allocator();

    // 最后激活内核地址空间，设置 satp 寄存器并刷新 TLB
    // 这会启用 RISC-V 的分页机制，所有内存访问将通过页表进行转换
    KERNEL_SPACE.exclusive_access().activate();
}
