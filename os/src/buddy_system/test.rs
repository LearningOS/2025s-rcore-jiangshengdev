use crate::buddy_system::linked_list;
use crate::buddy_system::linked_list::consume;

pub fn test_all() {
    test_linked_list();
}

fn test_linked_list() {
    let mut value1: usize = 0x5555;
    let mut value2: usize = 0x6666;
    let mut value3: usize = 0x7777;

    let mut list = linked_list::LinkedList::new();

    let list_addr = &list;

    let value1_addr = &mut value1 as *mut usize;
    let value2_addr = &mut value2 as *mut usize;
    let value3_addr = &mut value3 as *mut usize;

    consume(list_addr);

    unsafe {
        list.push(value1_addr);
        list.push(value2_addr);
        list.push(value3_addr);
    }

    assert_eq!(list.head, value3_addr);

    // Test links
    assert_eq!(value3, value2_addr as usize);
    assert_eq!(value2, value1_addr as usize);
    assert_eq!(value1, 0);

    // Test iter
    let mut iter = list.iter();
    assert_eq!(iter.next(), Some(&mut value3 as *mut usize));
    assert_eq!(iter.next(), Some(&mut value2 as *mut usize));
    assert_eq!(iter.next(), Some(&mut value1 as *mut usize));
    assert_eq!(iter.next(), None);

    // Test iter_mut

    // let mut iter_mut = list.iter_mut();
    // assert_eq!(iter_mut.next().unwrap().pop(), value3_addr);

    // Test pop
    let a = list.pop();
    let b = list.pop();
    let c = list.pop();
    let d = list.pop();

    assert_eq!(a, Some(value3_addr));
    assert_eq!(b, Some(value2_addr));
    assert_eq!(c, Some(value1_addr));
    assert_eq!(d, None);
}
