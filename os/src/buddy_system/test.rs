use crate::buddy_system::linked_list::consume;
use crate::buddy_system::{linked_list, Heap};
use core::alloc::Layout;
use core::mem::size_of;

pub fn test_all() {
    test_linked_list();
    test_linked_list_iter_mut();
    test_empty_heap();
    test_heap_add();
    test_heap_add_large();
    test_heap_oom();
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
    let mut heap = Heap::<32>::new();
    assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_err());

    let space: [usize; 100] = [0; 100];
    unsafe {
        heap.add_to_heap(space.as_ptr() as usize, space.as_ptr().add(100) as usize);
    }
    let addr = heap.alloc(Layout::from_size_align(1, 1).unwrap());
    assert!(addr.is_ok());
}

fn test_heap_add_large() {
    // Max size of block is 2^7 == 128 bytes
    let mut heap = Heap::<8>::new();
    assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_err());

    // 512 bytes of space
    let space: [u8; 512] = [0; 512];
    unsafe {
        heap.add_to_heap(space.as_ptr() as usize, space.as_ptr().add(512) as usize);
    }
    let addr = heap.alloc(Layout::from_size_align(1, 1).unwrap());
    assert!(addr.is_ok());
}

fn test_heap_oom() {
    let mut heap = Heap::<32>::new();
    let space: [usize; 100] = [0; 100];
    unsafe {
        heap.add_to_heap(space.as_ptr() as usize, space.as_ptr().add(100) as usize);
    }

    assert!(heap
        .alloc(Layout::from_size_align(100 * size_of::<usize>(), 1).unwrap())
        .is_err());
    assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_ok());
}

fn test_heap_alloc_and_free() {
    let mut heap = Heap::<32>::new();
    assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_err());

    let space: [usize; 100] = [0; 100];
    unsafe {
        heap.add_to_heap(space.as_ptr() as usize, space.as_ptr().add(100) as usize);
    }
    for _ in 0..100 {
        let addr = heap.alloc(Layout::from_size_align(1, 1).unwrap()).unwrap();
        heap.dealloc(addr, Layout::from_size_align(1, 1).unwrap());
    }
}
