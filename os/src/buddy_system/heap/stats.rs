use crate::buddy_system::Heap;

impl<const ORDER: usize> Heap<ORDER> {
    /// 返回用户实际请求的字节数
    #[allow(unused)]
    pub fn stats_alloc_user(&self) -> usize {
        self.user
    }

    /// 返回实际分配的字节数
    #[allow(unused)]
    pub fn stats_alloc_actual(&self) -> usize {
        self.allocated
    }

    /// 返回堆的总字节数
    #[allow(unused)]
    pub fn stats_total_bytes(&self) -> usize {
        self.total
    }
}
