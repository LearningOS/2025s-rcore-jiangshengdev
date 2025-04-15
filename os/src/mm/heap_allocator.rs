//! 全局分配器
//!
//! 实现了操作系统内核的堆内存分配器，使用 buddy_system_allocator 实现。
//! 这使得内核能够使用动态内存分配，如 Vec、Box 等 Rust 标准库中的数据结构。

use crate::config::KERNEL_HEAP_SIZE;
use buddy_system_allocator::LockedHeap;

#[global_allocator]
/// 堆分配器实例
///
/// 使用 buddy_system_allocator 中的 LockedHeap 作为全局分配器
/// 通过 #[global_allocator] 属性使其成为 Rust 程序的全局内存分配器
/// 初始状态为空，需要通过 init_heap 函数初始化

static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
/// 当堆分配错误发生时触发 panic
///
/// 该处理函数在堆内存分配失败时被调用，打印出失败的内存布局信息
/// 然后触发一个 panic，结束程序运行
///
/// # 参数
///
/// * `layout` - 分配失败时请求的内存布局信息

pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {

    // 显示内存分配请求的大小和对齐信息，然后触发 panic
    panic!("堆分配错误，布局 = {:?}", layout);
}

/// 堆空间 ([u8; KERNEL_HEAP_SIZE])
///
/// 静态分配的字节数组，用作内核堆的存储空间
/// 初始值全为零，大小由 KERNEL_HEAP_SIZE 常量决定
#[no_mangle]

static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

/// 初始化堆分配器
///
/// 该函数应在操作系统启动早期被调用，用于设置堆分配器
/// 将静态分配的 HEAP_SPACE 数组注册为堆分配器的管理内存
///
/// # 安全性
///
/// 该函数包含不安全代码，因为它：
/// 1. 访问可变静态变量 (HEAP_SPACE)
/// 2. 向分配器传递原始指针
///
/// 调用此函数前需确保它只被调用一次，且后续不会有对 HEAP_SPACE 的直接访问

pub fn init_heap() {

    // 使用不安全代码块，因为需要访问静态可变变量
    unsafe {

        // 获取堆空间的起始地址
        let start = HEAP_SPACE.as_ptr() as usize;

        // 获取堆空间的大小
        let size = KERNEL_HEAP_SIZE;

        // 初始化全局分配器，提供堆空间的地址和大小
        HEAP_ALLOCATOR.lock().init(start, size);
    }
}

#[allow(unused)]
/// 堆分配器的测试函数
///
/// 通过分配、使用和回收 Box 和 Vec 等结构来验证堆分配器的正确性
/// 还会检查分配的内存确实位于 BSS 段中
///
/// 测试流程：
/// 1. 分配一个 Box<usize> 并验证其值和位置
/// 2. 分配一个包含 500 个元素的 Vec<usize> 并验证其内容和位置
/// 3. 回收这些内存，确保不会发生泄漏或损坏

pub fn heap_test() {

    // 导入动态内存分配需要的数据结构
    use alloc::boxed::Box;
    use alloc::vec::Vec;

    extern "C" {

        // 获取 BSS 段的起始和结束地址
        // BSS 段用于存放未初始化的静态变量
        fn sbss();

        fn ebss();

    }

    // 计算 BSS 段的地址范围
    let bss_range = sbss as usize..ebss as usize;

    // 测试 1: 分配一个简单的 Box
    let a = Box::new(5);

    // 验证 Box 的值正确
    assert_eq!(*a, 5);

    // 验证分配的内存确实在 BSS 段中
    assert!(bss_range.contains(&(a.as_ref() as *const _ as usize)));

    // 手动释放 Box
    drop(a);

    // 测试 2: 分配一个 Vec 并填充元素
    let mut v: Vec<usize> = Vec::new();

    // 向 Vec 中添加 500 个元素
    for i in 0..500 {

        v.push(i);
    }

    // 验证 Vec 中的每个元素值正确
    for (i, val) in v.iter().take(500).enumerate() {

        assert_eq!(*val, i);
    }

    // 验证 Vec 的底层内存也在 BSS 段中
    assert!(bss_range.contains(&(v.as_ptr() as usize)));

    // 手动释放 Vec
    drop(v);

    println!("堆测试通过！");
}
