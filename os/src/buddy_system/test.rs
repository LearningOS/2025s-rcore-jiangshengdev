use crate::buddy_system::linked_list;
use crate::buddy_system::linked_list::consume;

pub fn test_all() {
    test_linked_list();
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

    // 4. “consume” 防止优化
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
