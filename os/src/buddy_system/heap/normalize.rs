use core::alloc::Layout;
use core::cmp::max;
use core::mem::size_of;

impl<const ORDER: usize> super::Heap<ORDER> {
    /// 统一计算符合对齐和 2 的幂要求的块大小及阶次索引
    #[inline]
    pub(super) fn normalize_layout(layout: &Layout) -> (usize, usize) {
        // 确保块大小满足最小对齐要求 (usize 或 layout 指定的对齐)
        let min_size_for_alignment = max(layout.align(), size_of::<usize>());

        // 确保块大小是 2 的幂，且能容纳请求的大小
        let size_as_power_of_two = layout.size().next_power_of_two();

        // 取两者中较大者，同时满足对齐和容量的 2 的幂要求
        let size = max(size_as_power_of_two, min_size_for_alignment);

        let order_idx = size.trailing_zeros() as usize;
        (size, order_idx)
    }
}
