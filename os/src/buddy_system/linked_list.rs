//! 提供侵入式链表（intrusive LinkedList）

use core::marker::PhantomData;
use core::{fmt, ptr};

pub fn nop() {}

pub fn consume<T>(_value: T) {}

/// 侵入式链表
///
/// 该实现参考了 CS140e 2018 Winter 课程中的链表实现，
///
/// 感谢 Sergio Benitez 的出色工作，
/// 详情见 [CS140e](https://cs140e.sergio.bz/)
#[derive(Copy, Clone)]
pub struct LinkedList {
    pub head: *mut usize,
}

unsafe impl Send for LinkedList {}

impl LinkedList {
    /// 创建一个新的 LinkedList
    pub const fn new() -> LinkedList {
        LinkedList {
            head: ptr::null_mut(),
        }
    }

    /// 如果链表为空则返回 `true`
    pub fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    /// 将 `item` 推入链表头部
    pub unsafe fn push(&mut self, item: *mut usize) {
        let next_addr = self.head as usize;
        *item = next_addr;
        self.head = item;
        nop();
    }

    /// 尝试移除链表头部的元素
    pub fn pop(&mut self) -> Option<*mut usize> {
        let empty = self.is_empty();

        match empty {
            true => None,
            false => {
                // 移动头指针
                let item = self.head;
                let next_addr = unsafe { *item as *mut usize };
                self.head = next_addr;
                Some(item)
            }
        }
    }

    /// 返回链表中元素的迭代器
    pub fn iter(&self) -> Iter {
        Iter {
            curr: self.head,
            list: PhantomData,
        }
    }

    #[allow(unused)]
    /// 返回链表中元素的可变迭代器
    pub fn iter_mut(&mut self) -> IterMut {
        IterMut {
            prev: &mut self.head as *mut *mut usize as *mut usize,
            curr: self.head,
            list: PhantomData,
        }
    }
}

impl fmt::Debug for LinkedList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// 链表的迭代器
pub struct Iter<'a> {
    curr: *mut usize,
    list: PhantomData<&'a LinkedList>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = *mut usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr.is_null() {
            None
        } else {
            let item = self.curr;
            let next = unsafe { *item as *mut usize };
            self.curr = next;
            Some(item)
        }
    }
}

#[allow(unused)]
/// 表示 `LinkedList` 中的可变节点
pub struct ListNode {
    prev: *mut usize,
    curr: *mut usize,
}

impl ListNode {
    #[allow(unused)]
    /// 将该节点从链表中移除
    pub fn pop(self) -> *mut usize {
        // 跳过当前节点
        unsafe {
            *(self.prev) = *(self.curr);
        }
        self.curr
    }

    #[allow(unused)]
    /// 返回节点指向的地址
    pub fn value(&self) -> *mut usize {
        self.curr
    }
}

/// 链表的可变迭代器
pub struct IterMut<'a> {
    list: PhantomData<&'a mut LinkedList>,
    prev: *mut usize,
    curr: *mut usize,
}

impl<'a> Iterator for IterMut<'a> {
    type Item = ListNode;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr.is_null() {
            None
        } else {
            let res = ListNode {
                prev: self.prev,
                curr: self.curr,
            };
            self.prev = self.curr;
            self.curr = unsafe { *self.curr as *mut usize };
            Some(res)
        }
    }
}
