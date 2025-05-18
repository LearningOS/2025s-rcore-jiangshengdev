use super::HeapError;
use core::alloc::Layout;
use core::ptr::NonNull;

impl<const ORDER: usize> super::Heap<ORDER> {
    /// 从堆中分配一段满足 `layout` 要求的内存
    pub fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, HeapError> {
        // 规范化后获取对应阶数的块并拆分
        let (block_size, target_order) = Self::normalize_layout(&layout);
        let ptr = self.acquire_block(target_order)?;
        // 更新统计信息：用户请求字节和实际分配字节
        self.update_stats_alloc(layout.size(), block_size);
        Ok(ptr)
    }

    /// 释放一段内存回堆
    pub fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        // 规范化后获取释放块的阶数和大小
        let (block_size, target_order) = Self::normalize_layout(&layout);
        unsafe {
            // 放回原阶并尝试向上合并伙伴块
            self.free_list[target_order].push(ptr.as_ptr() as *mut usize);
            self.merge_up(ptr.as_ptr() as usize, target_order);
        }
        // 更新统计信息：减去用户请求和实际分配字节
        self.update_stats_dealloc(layout.size(), block_size);
    }

    /// 从目标阶开始在空闲链表中查找并拆分至目标阶，返回分配块指针
    fn acquire_block(&mut self, target_order: usize) -> Result<NonNull<u8>, HeapError> {
        for current in target_order..self.free_list.len() {
            // 如果当前阶没有可用块，则跳过
            if self.free_list[current].is_empty() {
                continue;
            }
            unsafe {
                self.split_down(current, target_order)
                    .map_err(|_| HeapError::InternalError)?;
            }
            // 弹出拆分后的目标阶块
            let raw = self.free_list[target_order]
                .pop()
                .expect("should have free block") as *mut u8;
            return NonNull::new(raw).ok_or(HeapError::InternalError);
        }
        Err(HeapError::OutOfMemory)
    }

    /// 更新分配时的统计信息：user 增加请求字节，allocated 增加实际块大小
    fn update_stats_alloc(&mut self, user_size: usize, block_size: usize) {
        self.user += user_size;
        self.allocated += block_size;
    }

    /// 更新释放时的统计信息：user 减去请求字节，allocated 减去块大小
    fn update_stats_dealloc(&mut self, user_size: usize, block_size: usize) {
        self.user -= user_size;
        self.allocated -= block_size;
    }
}
