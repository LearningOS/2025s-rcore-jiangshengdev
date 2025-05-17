use core::cmp::min;

impl<const ORDER: usize> super::Heap<ORDER> {
    /// 将从 src_class 拆分到 dst_class
    pub(super) unsafe fn split_down(
        &mut self,
        mut src_class: usize,
        dst_class: usize,
    ) -> Result<(), ()> {
        while src_class > dst_class {
            // 取出一个大块
            let block = self.free_list[src_class].pop().ok_or(())?;
            let half = 1 << (src_class - 1);
            // 拆出左右两半
            self.free_list[src_class - 1].push((block as usize + half) as *mut usize);
            self.free_list[src_class - 1].push(block);
            src_class -= 1;
        }
        Ok(())
    }

    /// 从 ptr、class 开始，尝试向上合并伙伴块
    pub(super) unsafe fn merge_up(&mut self, mut ptr: usize, mut class: usize) {
        while class < self.free_list.len() - 1 {
            let buddy = ptr ^ (1 << class);
            // 在链表中查找伙伴
            let mut found = false;
            for node in self.free_list[class].iter_mut() {
                if node.value() as usize == buddy {
                    node.pop(); // 移出伙伴
                    found = true;
                    break;
                }
            }
            if !found {
                break;
            }
            // 移除当前块，更新 ptr/class，并插入上层链表
            self.free_list[class].pop();
            ptr = min(ptr, buddy);
            class += 1;
            self.free_list[class].push(ptr as *mut usize);
        }
    }
}
