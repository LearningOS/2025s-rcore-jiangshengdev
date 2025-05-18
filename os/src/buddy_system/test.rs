use crate::buddy_system::linked_list::consume;
use crate::buddy_system::{linked_list, Heap};
use core::alloc::Layout;
use core::mem::size_of;
use core::ptr::NonNull;

#[repr(align(4096))]
/// 4KB 页面对齐的 usize 数组，用于堆内存测试
struct Aligned127([usize; 127]);

#[repr(align(4096))]
/// 4KB 页面对齐的 u8 数组，用于堆内存测试
struct Aligned512([u8; 512]);

pub fn test_all() {
    println!("test_linked_list...");
    test_linked_list();
    println!("test_linked_list_iter_mut...");
    test_linked_list_iter_mut();
    println!("test_empty_heap...");
    test_empty_heap();
    println!("test_heap_add...");
    test_heap_add();
    println!("test_heap_add_large...");
    test_heap_add_large();
    println!("test_heap_oom...");
    test_heap_oom();
    println!("test_heap_alloc_and_free...");
    test_heap_alloc_and_free();
}

fn test_linked_list() {
    const N: usize = 16;
    const BASE: usize = 0x0000_0000_0000_0000;
    const STEP: usize = 0x1111_1111_1111_1111;

    // 1. 构建 16 个节点的值
    let mut values = [0usize; N];
    for (i, slot) in values.iter_mut().enumerate() {
        *slot = BASE + STEP * i;
    }

    // 2. 新建 list 并保存它的地址以便调试
    let mut list = linked_list::LinkedList::new();
    let list_addr = &list;
    // println!("list @ {:p}", list_addr);

    // 3. 构建指针数组
    let mut ptrs = [core::ptr::null_mut(); N];
    for (i, p) in ptrs.iter_mut().enumerate() {
        *p = &mut values[i] as *mut usize;
    }

    // 4. “consume”防止优化
    consume(list_addr);

    // 5. 依次 push
    unsafe {
        for &p in ptrs.iter() {
            list.push(p);
        }
    }

    // 6. head 应指向最后 push 的节点
    assert_eq!(list.head, ptrs[N - 1]);

    // 7. 检查内部 next 链接
    for i in (1..N).rev() {
        let next_addr = ptrs[i - 1] as usize;
        assert_eq!(unsafe { *ptrs[i] }, next_addr);
    }
    assert_eq!(unsafe { *ptrs[0] }, 0);

    // 8. 迭代器也应按 LIFO 顺序给出同样的指针序列
    let mut it = list.iter();
    for &expect in ptrs.iter().rev() {
        assert_eq!(it.next(), Some(expect));
    }
    assert!(it.next().is_none());

    // 9. pop 也应以同样顺序逐个拿出
    for &expect in ptrs.iter().rev() {
        assert_eq!(list.pop().unwrap(), expect);
    }
    assert!(list.pop().is_none());
}

fn test_linked_list_iter_mut() {
    // 构建链表并插入若干节点
    const N: usize = 8;
    let mut values = [0usize; N];
    let mut list = linked_list::LinkedList::new();
    let mut ptrs = [core::ptr::null_mut(); N];
    for (i, slot) in values.iter_mut().enumerate() {
        ptrs[i] = slot as *mut usize;
    }
    unsafe {
        for &p in ptrs.iter() {
            list.push(p);
        }
    }
    // 选择一个指针，模拟实际分配器用法
    let target_ptr = ptrs[3];
    let mut found = false;
    for node in list.iter_mut() {
        if node.value() == target_ptr {
            let p = node.pop();
            assert_eq!(p, target_ptr);
            found = true;
            break;
        }
    }
    assert!(found, "未找到目标节点");
    // 检查链表中不再有该指针，其余指针都还在
    for node in list.iter() {
        assert_ne!(node, target_ptr);
    }
    let remain = list.iter().count();
    assert_eq!(remain, N - 1);
}

fn test_empty_heap() {
    let mut heap = Heap::<32>::new();
    assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_err());
}

fn test_heap_add() {
    // 确保初始无可用空间
    let mut heap = Heap::<32>::new();
    assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_err());
    // 使用 4KB 对齐的 usize 数组作为堆内存区域
    let space = Aligned127([0x5555_5555_5555_5555; 127]);
    unsafe {
        heap.add_to_heap(
            space.0.as_ptr() as usize,
            space.0.as_ptr().add(127) as usize,
        );
    }
    let addr = heap.alloc(Layout::from_size_align(1, 1).unwrap());
    assert!(addr.is_ok());
}

fn test_heap_add_large() {
    // 确保初始无可用空间
    let mut heap = Heap::<8>::new();
    assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_err());
    // 使用 4KB 对齐的 u8 数组作为堆内存区域
    let space = Aligned512([0x5; 512]);
    unsafe {
        heap.add_to_heap(
            space.0.as_ptr() as usize,
            space.0.as_ptr().add(512) as usize,
        );
    }
    let addr = heap.alloc(Layout::from_size_align(1, 1).unwrap());
    assert!(addr.is_ok());
}

fn test_heap_oom() {
    let mut heap = Heap::<32>::new();
    // 使用 4KB 对齐的 usize 数组作为堆内存区域
    let space = Aligned127([0x5555_5555_5555_5555; 127]);
    unsafe {
        heap.add_to_heap(
            space.0.as_ptr() as usize,
            space.0.as_ptr().add(127) as usize,
        );
    }

    // 分配请求大于堆空间，预期分配失败（OOM）
    assert!(heap
        .alloc(Layout::from_size_align(129 * size_of::<usize>(), 1).unwrap())
        .is_err());
    assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_ok());
}

fn test_heap_alloc_and_free() {
    let mut heap = Heap::<32>::new();
    assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_err());

    // 使用 4KB 对齐的 usize 数组作为堆内存区域
    let space = Aligned127([0x5555_5555_5555_5555; 127]);
    unsafe {
        heap.add_to_heap(
            space.0.as_ptr() as usize,
            space.0.as_ptr().add(127) as usize,
        );
    }

    println!("{:#?}", heap);

    // 先统一分配 127 次，再统一释放
    let mut addrs: [NonNull<u8>; 127] = [NonNull::dangling(); 127];
    for i in 0..127 {
        addrs[i] = heap.alloc(Layout::from_size_align(1, 1).unwrap()).unwrap();
    }

    println!("{:#?}", heap);

    for &addr in addrs.iter() {
        heap.dealloc(addr, Layout::from_size_align(1, 1).unwrap());
    }

    println!("{:#?}", heap);
}
