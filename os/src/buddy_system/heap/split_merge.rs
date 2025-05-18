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
            // 取出一个大块
            let block = self.free_list[src_order_idx]
                .pop()
                .ok_or(HeapError::InternalError)?;
            let half = 1 << (src_order_idx - 1);
            // 拆出左右两半
            self.free_list[src_order_idx - 1].push((block as usize + half) as *mut usize);
            self.free_list[src_order_idx - 1].push(block);
            src_order_idx -= 1;
        }
        Ok(())
    }

    /// 从 ptr、order_idx 开始，尝试向上合并伙伴块
    pub(super) unsafe fn merge_up(&mut self, mut ptr: usize, mut order_idx: usize) {
        while order_idx < self.free_list.len() - 1 {
            let buddy = ptr ^ (1 << order_idx);
            // 在链表中查找伙伴
            let mut found = false;
            for node in self.free_list[order_idx].iter_mut() {
                if node.value() as usize == buddy {
                    node.pop(); // 移出伙伴
                    found = true;
                    break;
                }
            }
            if !found {
                break;
            }
            // 移除当前块，更新 ptr/order_idx，并插入上层链表
            self.free_list[order_idx].pop();
            ptr = min(ptr, buddy);
            order_idx += 1;
            self.free_list[order_idx].push(ptr as *mut usize);
        }
    }
}
