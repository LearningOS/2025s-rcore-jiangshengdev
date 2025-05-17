use crate::buddy_system::linked_list;
use util::prev_power_of_two;

/// 伙伴系统堆，最大阶数为 `ORDER - 1`
pub struct Heap<const ORDER: usize> {
    // 伙伴系统的空闲链表，每个阶对应一个链表
    free_list: [linked_list::LinkedList; ORDER],

    // 统计信息
    pub(crate) user: usize,      // 用户实际请求的字节数
    pub(crate) allocated: usize, // 实际分配的字节数
    pub(crate) total: usize,     // 堆总字节数
}

mod add;
mod alloc;
mod ctor;
mod debug;
pub mod heap_tests;
mod normalize;
mod split_merge;
mod stats;
mod util;
