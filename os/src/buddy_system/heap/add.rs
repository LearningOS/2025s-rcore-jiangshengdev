use crate::buddy_system::heap::util::{align_down, align_up};
use core::mem::size_of;

impl<const ORDER: usize> super::Heap<ORDER> {
    /// 向堆中添加一段内存区间 [start, end)
    pub unsafe fn add_to_heap(&mut self, start: usize, end: usize) {
        // 对输入区间进行对齐和校验
        let (start, end) = self.align_and_validate(start, end);
        // 拆分对齐后的区间并插入空闲链表，获取累积添加字节数
        let added = self.split_and_push(start, end);
        // 更新堆总体统计
        self.total += added;
    }

    /// 对输入区间进行对齐并验证，返回对齐后的 `(start, end)`
    #[inline]
    fn align_and_validate(&self, mut start: usize, mut end: usize) -> (usize, usize) {
        // usize 对齐边界，用于确保地址以指针大小对齐
        let ptr_align = size_of::<usize>();
        // 将 start 向上对齐到指针对齐边界
        start = align_up(start, ptr_align);
        // 将 end 向下对齐到指针对齐边界
        end = align_down(end, ptr_align);
        // 验证对齐后区间为有效范围
        assert!(
            start <= end,
            "add_to_heap: start ({:#x}) > end ({:#x})",
            start,
            end
        );
        (start, end)
    }

    /// 拆分对齐后的区间并插入对应阶的空闲链表，返回累积字节数
    #[inline]
    unsafe fn split_and_push(&mut self, start: usize, end: usize) -> usize {
        // usize 对齐边界，用于循环结束条件
        let ptr_align = size_of::<usize>();
        let mut total_added = 0;
        let mut current = start;
        while current + ptr_align <= end {
            // 计算块大小与阶次
            let (block_size, order) = self.calc_block_size_and_order(current, end - current);
            // 累加统计
            total_added += block_size;
            // 将该块插入对应阶的空闲链表
            self.free_list[order].push(current as *mut usize);
            // 前进到下一个块起始位置
            current += block_size;
        }
        total_added
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
