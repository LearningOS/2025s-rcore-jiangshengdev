use core::alloc::Layout;
use core::cmp::max;
use core::mem::size_of;

impl<const ORDER: usize> super::Heap<ORDER> {
    /// 统一计算符合对齐和 2 的幂要求的块大小及阶次
    #[inline]
    pub(super) fn normalize_layout(layout: &Layout) -> (usize, usize) {
        let size = max(
            layout.size().next_power_of_two(),
            max(layout.align(), size_of::<usize>()),
        );
        let class = size.trailing_zeros() as usize;
        (size, class)
    }
}
