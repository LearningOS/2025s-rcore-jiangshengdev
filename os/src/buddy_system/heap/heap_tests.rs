//! 手动调用的边界和极端情况测试（no_std + core）
use super::Heap;
use core::alloc::Layout;

// 1. 最小对齐测试
fn test_min_alignment() {
    let mut heap = Heap::<4>::new();
    let buf = [0u8; 24];
    // 8 字节对齐
    let start = (buf.as_ptr() as usize + 7) & !7;
    let end = (unsafe { buf.as_ptr().add(buf.len()) } as usize) & !7;
    unsafe {
        heap.add_to_heap(start, end);
    }
    let layout = Layout::from_size_align(1, 8).unwrap();
    let ptr = heap.alloc(layout).unwrap();
    assert_eq!((ptr.as_ptr() as usize) % 8, 0);
}

// 2. 拆分下行测试
fn test_split_down() {
    let mut heap = Heap::<5>::new();
    let buf = [0u8; 64];
    unsafe {
        heap.add_to_heap(buf.as_ptr() as usize, buf.as_ptr().add(buf.len()) as usize);
    }
    let before = heap.stats_total_bytes();
    let _ = heap.alloc(Layout::from_size_align(1, 1).unwrap()).unwrap();
    // 分配小块后，total 不变，allocated 增加
    assert_eq!(heap.stats_total_bytes(), before);
}

// 3. 合并上行测试
fn test_merge_up() {
    let mut heap = Heap::<4>::new();
    let buf = [0u8; 32];
    unsafe {
        heap.add_to_heap(buf.as_ptr() as usize, buf.as_ptr().add(buf.len()) as usize);
    }
    // 连续分配两个最小块
    let a = heap.alloc(Layout::from_size_align(1, 1).unwrap()).unwrap();
    let _b = heap.alloc(Layout::from_size_align(1, 1).unwrap()).unwrap();
    let used = heap.stats_alloc_actual();
    // 释放它们，应合并
    heap.dealloc(_b, Layout::from_size_align(1, 1).unwrap());
    heap.dealloc(a, Layout::from_size_align(1, 1).unwrap());
    assert!(heap.stats_alloc_actual() < used);
}

// 4. 阶数溢出测试
fn test_order_overflow() {
    let mut heap = Heap::<4>::new();
    let buf = [0u8; 24];
    let start = (buf.as_ptr() as usize + 7) & !7;
    let min_block_size = 1 << (4 - 1); // 8 bytes for ORDER = 4
    let potential_end = (unsafe { buf.as_ptr().add(buf.len()) } as usize) & !7;
    let end = if potential_end >= start + min_block_size {
        potential_end
    } else {
        start
    };

    if start < end {
        // 只有当有有效空间时才添加到堆
        unsafe {
            heap.add_to_heap(start, end);
        }
        // 能至少分配一个符合 ORDER 的块，这里请求 4 字节，对齐 4 字节
        // 对于 ORDER=4，最小块是 8 字节，所以分配 4 字节应该没问题，会被提升到 8 字节。
        assert!(heap.alloc(Layout::from_size_align(4, 4).unwrap()).is_ok());
    } else {
        // 如果对齐后没有有效空间，则预期分配失败或不进行分配
        assert!(heap.alloc(Layout::from_size_align(4, 4).unwrap()).is_err());
    }
}

// 5. OOM 重试测试
fn test_oom_retry() {
    let mut heap = Heap::<5>::new();

    // 第一个内存区域
    let buf_small_backing = [0u8; 24]; // 提供足够大的后备缓冲区以容纳对齐和原始大小
    let ptr_small_raw = buf_small_backing.as_ptr() as usize;
    let start_small = (ptr_small_raw + 7) & !7; // 确保 start_small 是 8 字节对齐的
    let end_small = start_small + 8; // 明确指定一个 8 字节长度的对齐区域

    // 确保请求的区域在后备缓冲区内
    assert!(
        end_small <= ptr_small_raw + buf_small_backing.len(),
        "Test setup error: insufficient space in buf_small_backing after alignment."
    );
    unsafe {
        heap.add_to_heap(start_small, end_small);
    }

    assert!(heap.alloc(Layout::from_size_align(16, 1).unwrap()).is_err());

    // 第二个内存区域
    let buf_more_backing = [0u8; 128]; // 提供足够大的后备缓冲区 (确保能容纳 64 字节对齐的 64 字节块)
    let ptr_more_raw = buf_more_backing.as_ptr() as usize;
    let start_more = (ptr_more_raw + 63) & !63; // 确保 start_more 是 64 字节对齐的
    let end_more = start_more + 64; // 明确指定一个 64 字节长度的对齐区域

    // 确保请求的区域在后备缓冲区内
    assert!(
        end_more <= ptr_more_raw + buf_more_backing.len(),
        "Test setup error: insufficient space in buf_more_backing after alignment."
    );
    unsafe {
        heap.add_to_heap(start_more, end_more);
    }

    assert!(heap.alloc(Layout::from_size_align(16, 1).unwrap()).is_ok());
}

// 6. 零大小请求测试
fn test_zero_size_alloc() {
    let mut heap = Heap::<4>::new();
    let buf = [0u8; 16];
    unsafe {
        heap.add_to_heap(buf.as_ptr() as usize, buf.as_ptr().add(buf.len()) as usize);
    }
    let layout = Layout::from_size_align(0, 1).unwrap();
    let result = heap.alloc(layout);
    // 分配器将零大小请求向上取整到其最小块大小（8 字节）
    // 并且如果该大小的块可用，则成功分配。
    assert!(result.is_ok());
}

// 7. 对齐大于大小测试
fn test_align_gt_size() {
    let mut heap = Heap::<4>::new();
    let buf = [0u8; 16];
    // 8 字节对齐
    let start = (buf.as_ptr() as usize + 7) & !7;
    let end = (unsafe { buf.as_ptr().add(buf.len()) } as usize) & !7;
    unsafe {
        heap.add_to_heap(start, end);
    }
    let layout = Layout::from_size_align(4, 8).unwrap();
    assert!(heap.alloc(layout).is_ok());
}

// 8. 碎片化测试
fn test_fragmentation() {
    let mut heap = Heap::<5>::new();
    let buf = [0u8; 64];
    // 8 字节对齐
    let start = (buf.as_ptr() as usize + 7) & !7;
    let end = (unsafe { buf.as_ptr().add(buf.len()) } as usize) & !7;
    unsafe {
        heap.add_to_heap(start, end);
    }
    let a = heap.alloc(Layout::from_size_align(8, 8).unwrap()).unwrap();
    let _b = heap.alloc(Layout::from_size_align(8, 8).unwrap()).unwrap();
    heap.dealloc(a, Layout::from_size_align(8, 8).unwrap());
    let c = heap.alloc(Layout::from_size_align(4, 4).unwrap()).unwrap();
    // 新分配的块应复用 a 的空间
    assert_eq!(c.as_ptr() as usize, a.as_ptr() as usize);
}

// 9. 耗尽空间测试
fn test_exhaust_all_space() {
    const ORDER: usize = 4;
    let mut heap = Heap::<ORDER>::new();
    let buf = [0u8; 16];
    // 8 字节对齐
    let start = (buf.as_ptr() as usize + 7) & !7;
    let end = (unsafe { buf.as_ptr().add(buf.len()) } as usize) & !7;
    unsafe {
        heap.add_to_heap(start, end);
    }
    let mut count = 0;
    while heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_ok() {
        count += 1;
    }
    let total_bytes = heap.stats_total_bytes();
    let min_block_size = 1 << (ORDER - 1);
    assert_eq!(
        count,
        total_bytes / min_block_size,
        "alloc count * min_block_size must exhaust total bytes"
    );
}

/// 手动运行所有新边界测试
pub fn test_heap_edge_all() {
    println!("test_min_alignment...");
    test_min_alignment();
    println!("test_split_down...");
    test_split_down();
    println!("test_merge_up...");
    test_merge_up();
    println!("test_order_overflow...");
    test_order_overflow();
    println!("test_oom_retry...");
    test_oom_retry();
    println!("test_zero_size_alloc...");
    test_zero_size_alloc();
    println!("test_align_gt_size...");
    test_align_gt_size();
    println!("test_fragmentation...");
    test_fragmentation();
    println!("test_exhaust_all_space...");
    test_exhaust_all_space();
}
