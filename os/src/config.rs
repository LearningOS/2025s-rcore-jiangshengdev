//! 内核常量。

#[allow(unused)]

/// 用户应用栈大小。
pub const USER_STACK_SIZE: usize = 4096 * 2;

/// 内核栈大小。
pub const KERNEL_STACK_SIZE: usize = 4096 * 4;

/// 内核堆大小。
pub const KERNEL_HEAP_SIZE: usize = 0x200_0000;

/// 页大小：4KB。
pub const PAGE_SIZE: usize = 0x1000;

/// 页大小的比特数：12。
pub const PAGE_SIZE_BITS: usize = 0xc;

/// trampoline 的虚拟地址。
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;

/// trap context 的虚拟地址。
pub const TRAP_CONTEXT_BASE: usize = TRAMPOLINE - PAGE_SIZE;

/// 时钟频率。
pub const CLOCK_FREQ: usize = 12500000;

/// 物理内存结束地址。
pub const MEMORY_END: usize = 0x88000000;
