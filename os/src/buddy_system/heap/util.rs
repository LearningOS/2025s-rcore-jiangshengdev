/// 计算小于等于 num 的最大 2 的幂
pub fn prev_power_of_two(num: usize) -> usize {
    1 << (usize::BITS as usize - num.leading_zeros() as usize - 1)
}

/// 向上对齐地址 `addr` 到 `align` 的整数倍
pub fn align_up(addr: usize, align: usize) -> usize {
    let mask = !(align - 1);
    (addr + align - 1) & mask
}

/// 向下对齐地址 `addr` 到 `align` 的整数倍
pub fn align_down(addr: usize, align: usize) -> usize {
    addr & (!(align - 1))
}

/// 计算指定阶的块大小，即 2^order_idx
pub const fn block_size(order_idx: usize) -> usize {
    1 << order_idx
}

/// 计算指定阶的半块大小，即 2^(order_idx-1)
pub const fn half_block_size(order_idx: usize) -> usize {
    1 << (order_idx - 1)
}
