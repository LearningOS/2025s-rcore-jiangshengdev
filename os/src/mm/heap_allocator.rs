//! 全局分配器

use crate::config::KERNEL_HEAP_SIZE;
use buddy_system_allocator::LockedHeap;

#[global_allocator]
/// 堆分配器实例

static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
/// 当堆分配错误发生时触发 panic

pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {

    panic!("堆分配错误，布局 = {:?}", layout);
}

/// 堆空间 ([u8; KERNEL_HEAP_SIZE])

static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

/// 初始化堆分配器

pub fn init_heap() {

    unsafe {

        let start = HEAP_SPACE.as_ptr() as usize;

        let size = KERNEL_HEAP_SIZE;

        HEAP_ALLOCATOR.lock().init(start, size);
    }
}

#[allow(unused)]

pub fn heap_test() {

    use alloc::boxed::Box;
    use alloc::vec::Vec;

    extern "C" {

        fn sbss();

        fn ebss();

    }

    let bss_range = sbss as usize..ebss as usize;

    let a = Box::new(5);

    assert_eq!(*a, 5);

    assert!(bss_range.contains(&(a.as_ref() as *const _ as usize)));

    drop(a);

    let mut v: Vec<usize> = Vec::new();

    for i in 0..500 {

        v.push(i);
    }

    for (i, val) in v.iter().take(500).enumerate() {

        assert_eq!(*val, i);
    }

    assert!(bss_range.contains(&(v.as_ptr() as usize)));

    drop(v);

    println!("堆测试通过！");
}
