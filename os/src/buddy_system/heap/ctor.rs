use crate::buddy_system::linked_list::LinkedList;

impl<const ORDER: usize> super::Heap<ORDER> {
    /// 创建一个空堆
    pub const fn new() -> Self {
        Self {
            free_list: [LinkedList::new(); ORDER],
            user: 0,
            allocated: 0,
            total: 0,
        }
    }

    /// 创建一个空堆（别名）
    #[allow(unused)]
    pub const fn empty() -> Self {
        Self::new()
    }
}
