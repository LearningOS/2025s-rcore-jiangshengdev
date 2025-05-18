use super::HeapError;
use core::alloc::Layout;
use core::ptr::NonNull;

impl<const ORDER: usize> super::Heap<ORDER> {
    /// 从堆中分配一段满足 `layout` 要求的内存
    pub fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, HeapError> {
        // 计算满足对齐和 2^k 要求的块大小及阶次索引
        let (size, order_idx) = Self::normalize_layout(&layout);
        // 从目标阶开始在空闲链表中查找可用块
        for search_order_idx in order_idx..self.free_list.len() {
            if !self.free_list[search_order_idx].is_empty() {
                unsafe {
                    // 将更高阶的块拆分到目标阶
                    self.split_down(search_order_idx, order_idx)
                        .map_err(|_| HeapError::InternalError)?;
                }
                // 弹出目标阶块
                let ptr = NonNull::new(
                    self.free_list[order_idx]
                        .pop()
                        .expect("should have free block") as *mut u8,
                )
                .ok_or(HeapError::InternalError)?;
                // 更新统计信息
                self.user += layout.size();
                self.allocated += size;
                return Ok(ptr);
            }
        }
        Err(HeapError::OutOfMemory)
    }

    /// 释放一段内存回堆
    pub fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        // 计算块大小及阶次索引
        let (size, order_idx) = Self::normalize_layout(&layout);
        unsafe {
            // 放回原阶空闲链表
            self.free_list[order_idx].push(ptr.as_ptr() as *mut usize);
            // 尝试与伙伴块向上合并
            self.merge_up(ptr.as_ptr() as usize, order_idx);
        }
        // 更新统计信息
        self.user -= layout.size();
        self.allocated -= size;
    }
}
