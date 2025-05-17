//! 提供侵入式链表（intrusive LinkedList）

use core::marker::PhantomData;
use core::{fmt, ptr};

/// 指向链表节点的指针类型别名
type NodePtr = *mut usize;

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
    pub head: NodePtr,
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

    /// 将 `node` 推入链表头部
    pub unsafe fn push(&mut self, node: NodePtr) {
        let old_head = self.head;
        *node = old_head as usize;
        self.head = node;
        nop();
    }

    /// 尝试移除链表头部的元素
    pub fn pop(&mut self) -> Option<NodePtr> {
        let empty = self.is_empty();

        match empty {
            true => None,
            false => {
                // 移动头指针
                let node: NodePtr = self.head;
                let new_head: NodePtr = unsafe { *node as NodePtr };
                self.head = new_head;
                Some(node)
            }
        }
    }

    /// 返回链表中元素的迭代器
    pub fn iter(&self) -> Iter {
        Iter {
            curr_ptr: self.head,
            list: PhantomData,
        }
    }

    /// 返回链表中元素的可变迭代器
    pub fn iter_mut(&mut self) -> IterMut {
        let head_ptr = &mut self.head as *mut NodePtr as NodePtr;
        IterMut {
            prev_ptr: head_ptr,
            curr_ptr: self.head,
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
    curr_ptr: NodePtr,
    list: PhantomData<&'a LinkedList>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = NodePtr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr_ptr.is_null() {
            None
        } else {
            let node = self.curr_ptr;
            let next_ptr = unsafe { *node as NodePtr };
            self.curr_ptr = next_ptr;
            Some(node)
        }
    }
}

/// 表示 `LinkedList` 中的可变节点
pub struct ListNode {
    prev_ptr: NodePtr,
    curr_ptr: NodePtr,
}

impl ListNode {
    /// 将该节点从链表中移除
    pub fn pop(self) -> NodePtr {
        // 跳过当前节点
        unsafe {
            *(self.prev_ptr) = *(self.curr_ptr);
        }
        self.curr_ptr
    }

    /// 返回节点指向的地址
    pub fn value(&self) -> NodePtr {
        self.curr_ptr
    }
}

/// 链表的可变迭代器
pub struct IterMut<'a> {
    list: PhantomData<&'a mut LinkedList>,
    prev_ptr: NodePtr,
    curr_ptr: NodePtr,
}

impl<'a> Iterator for IterMut<'a> {
    type Item = ListNode;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr_ptr.is_null() {
            None
        } else {
            let node = ListNode {
                prev_ptr: self.prev_ptr,
                curr_ptr: self.curr_ptr,
            };
            self.prev_ptr = self.curr_ptr;
            let next_ptr = unsafe { *self.curr_ptr as NodePtr };
            self.curr_ptr = next_ptr;
            Some(node)
        }
    }
}
