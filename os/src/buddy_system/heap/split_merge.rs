use super::util::{block_size, half_block_size};
use super::HeapError;
use core::cmp::min;

impl<const ORDER: usize> super::Heap<ORDER> {
    /// 将从 src_order_idx 拆分到 dst_order_idx
    pub(super) unsafe fn split_down(
        &mut self,
        mut src_order_idx: usize,
        dst_order_idx: usize,
    ) -> Result<(), HeapError> {
        while src_order_idx > dst_order_idx {
            // 从 free_list 弹出一个块并拆分为两半
            let block_ptr = self.pop_block(src_order_idx)?;
            let split_size = half_block_size(src_order_idx);
            self.push_block(src_order_idx - 1, block_ptr as usize + split_size);
            self.push_block(src_order_idx - 1, block_ptr as usize);
            src_order_idx -= 1;
        }
        Ok(())
    }

    /// 从 ptr、order_idx 开始，尝试向上合并伙伴块
    pub(super) unsafe fn merge_up(&mut self, mut ptr: usize, mut order_idx: usize) {
        // 重复尝试合并，直到无法再合并
        while self.try_merge_step(&mut ptr, &mut order_idx) {}
    }

    /// 弹出指定阶的空闲块，若为空返回 InternalError
    unsafe fn pop_block(&mut self, order_idx: usize) -> Result<*mut usize, HeapError> {
        self.free_list[order_idx].pop().ok_or(HeapError::InternalError)
    }

    /// 将地址 addr 插入指定阶的空闲链表
    unsafe fn push_block(&mut self, order_idx: usize, addr: usize) {
        self.free_list[order_idx].push(addr as *mut usize);
    }

    /// 在指定阶链表中查找并移除伙伴块，找到返回 true
    unsafe fn find_and_remove_buddy(&mut self, order_idx: usize, buddy_addr: usize) -> bool {
        for node in self.free_list[order_idx].iter_mut() {
            if node.value() as usize == buddy_addr {
                node.pop();
                return true;
            }
        }
        false
    }

    /// 尝试执行一次向上合并，找到则合并并返回 true，否则返回 false
    unsafe fn try_merge_step(&mut self, ptr: &mut usize, order_idx: &mut usize) -> bool {
        if *order_idx >= self.free_list.len() - 1 {
            return false;
        }
        let current_size = block_size(*order_idx);
        let buddy = *ptr ^ current_size;
        if !self.find_and_remove_buddy(*order_idx, buddy) {
            return false;
        }
        // 移除当前块
        self.free_list[*order_idx].pop();
        // 更新为较小地址、阶数+1 并插入上阶链表
        *ptr = min(*ptr, buddy);
        *order_idx += 1;
        self.push_block(*order_idx, *ptr);
        true
    }
}
