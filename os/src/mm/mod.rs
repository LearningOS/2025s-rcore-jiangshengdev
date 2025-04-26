//! Memory management implementation
//!
//! SV39 page-based virtual-memory architecture for RV64 systems, and
//! everything about memory management, like frame allocator, page table,
//! map area and memory set, is implemented here.
//!
//! Every task or process has a memory_set to control its virtual memory.
mod address;
mod frame_allocator;
mod heap_allocator;
mod map;
mod memory_set;
mod mmap_area;
mod page_table;
mod user;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use address::{StepByOne, VPNRange};
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use map::parse_prot_flags;
pub use memory_set::remap_test;
pub use memory_set::{MapPermission, MemorySet, KERNEL_SPACE};
pub use mmap_area::MmapAreaManager;
pub use page_table::{translated_byte_buffer, translated_refmut, translated_str, PageTableEntry};
use page_table::{PTEFlags, PageTable};
pub use user::write_user_struct;
/// initiate heap allocator, frame allocator and kernel space
pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.exclusive_access().activate();
}
