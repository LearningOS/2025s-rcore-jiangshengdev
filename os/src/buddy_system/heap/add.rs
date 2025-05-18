use crate::buddy_system::heap::util::{align_down, align_up};
use core::mem::size_of;

impl<const ORDER: usize> super::Heap<ORDER> {
    /// 向堆中添加一段内存区间 [start, end)
    pub unsafe fn add_to_heap(&mut self, mut start: usize, mut end: usize) {
        // 对 start 进行上对齐，保证地址符合 usize 对齐要求
        start = align_up(start, size_of::<usize>());
        // 对 end 进行下对齐，保证地址符合 usize 对齐要求
        end = align_down(end, size_of::<usize>());
        // 确保对齐后起始地址不大于结束地址
        assert!(start <= end);

        // 本次加入堆的总字节数统计
        let mut total = 0;
        // 当前待分割块的起始地址
        let mut current_start = start;

        // 循环将区间按伙伴系统要求切分为若干 2^k 块
        while current_start + size_of::<usize>() <= end {
            // 计算当前可分配块的大小及阶次索引
            let (size, order_idx) =
                self.calc_block_size_and_order(current_start, end - current_start);
            // 累加统计
            total += size;

            // 将块插入对应阶的空闲链表
            self.free_list[order_idx].push(current_start as *mut usize);
            // 前进到下一个块
            current_start += size;
        }

        // 更新堆总体统计
        self.total += total;
    }

    /// 向堆中添加一段内存区间 [start, start+size)
    #[allow(unused)]
    pub unsafe fn init(&mut self, start: usize, size: usize) {
        self.add_to_heap(start, start + size);
    }

    /// 根据地址和剩余空间计算块大小与阶次
    #[inline]
    fn calc_block_size_and_order(&self, addr: usize, remaining: usize) -> (usize, usize) {
        // 计算 addr 能向下对齐到的最大 2 的幂 (确保块地址是其大小的倍数)
        let max_aligned_size_at_addr = addr & (!addr + 1);

        // 计算小于等于 remaining 的最大 2 的幂 (剩余空间能容纳的最大理想块)
        let max_pow2_in_remaining = super::prev_power_of_two(remaining);

        // 取两者中较小者，保证地址对齐且不超过剩余空间
        let mut size = core::cmp::min(max_aligned_size_at_addr, max_pow2_in_remaining);
        // 阶次索引由块大小的 trailing zeros 得到
        let mut order_idx = size.trailing_zeros() as usize;
        // 若超出最大阶，则使用最大阶对应的块大小
        if order_idx > ORDER - 1 {
            order_idx = ORDER - 1;
            size = 1 << order_idx;
        }
        (size, order_idx)
    }
}
