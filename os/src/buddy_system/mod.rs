mod linked_list;
mod test;

use core::alloc::Layout;
use core::cmp::{max, min};
use core::fmt;
use core::mem::size_of;
use core::ptr::NonNull;
pub use test::test_all;

/// 伙伴系统堆，最大阶数为 `ORDER - 1`
pub struct Heap<const ORDER: usize> {
    // 伙伴系统的空闲链表，每个阶对应一个链表
    free_list: [linked_list::LinkedList; ORDER],

    // 统计信息
    user: usize,      // 用户实际请求的字节数
    allocated: usize, // 实际分配的字节数
    total: usize,     // 堆总字节数
}

impl<const ORDER: usize> Heap<ORDER> {
    /// 创建一个空堆
    pub const fn new() -> Self {
        Heap {
            free_list: [linked_list::LinkedList::new(); ORDER],
            user: 0,
            allocated: 0,
            total: 0,
        }
    }

    /// 创建一个空堆（别名）
    #[allow(unused)]
    pub const fn empty() -> Self {
        Self::new()
    }

    /// 向堆中添加一段内存区间 [start, end)
    pub unsafe fn add_to_heap(&mut self, mut start: usize, mut end: usize) {
        // 对 start 进行上对齐，保证地址符合 usize 对齐要求
        start = (start + size_of::<usize>() - 1) & (!size_of::<usize>() + 1);
        // 对 end 进行下对齐，保证地址符合 usize 对齐要求
        end &= !size_of::<usize>() + 1;
        // 确保对齐后起始地址不大于结束地址
        assert!(start <= end);

        // 用于统计本次加入堆的总字节数
        let mut total = 0;
        // 当前处理的内存块起始地址
        let mut current_start = start;

        // 循环分割区间，将其按伙伴系统要求切分为若干大小为 2^k 的块
        while current_start + size_of::<usize>() <= end {
            // 计算当前地址的 lowbit，用于确定可分配的最大块对齐边界
            let lowbit = current_start & (!current_start + 1);
            // 可分配块大小为 lowbit 与剩余空间最大 2 的幂的较小者
            let mut size = min(lowbit, prev_power_of_two(end - current_start));

            // 根据块大小计算其阶数 k
            let mut order = size.trailing_zeros() as usize;
            // 如果阶数超出最大阶，则使用最大阶并调整块大小
            if order > ORDER - 1 {
                order = ORDER - 1;
                size = 1 << order;
            }
            // 累加此块字节数到 total
            total += size;

            // 将当前块插入对应阶的空闲链表
            self.free_list[order].push(current_start as *mut usize);
            // 移动到下一个块的起始地址
            current_start += size;
        }

        // 更新堆的总字节数统计
        self.total += total;
    }

    /// 向堆中添加一段内存区间 [start, start+size)
    #[allow(unused)]
    pub unsafe fn init(&mut self, start: usize, size: usize) {
        self.add_to_heap(start, start + size);
    }

    /// 从堆中分配一段满足 `layout` 要求的内存
    pub fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, ()> {
        // 取满足大小、对齐和 usize 对齐要求的最小 2 的幂
        let size = max(
            layout.size().next_power_of_two(),
            max(layout.align(), size_of::<usize>()),
        );
        // 计算阶次 class = log2(size)
        let class = size.trailing_zeros() as usize;
        // 从目标阶开始，寻找可以分配的空闲块
        for i in class..self.free_list.len() {
            // 找到第一个非空的链表
            if !self.free_list[i].is_empty() {
                // 如果是在更高阶找到的块，则拆分到目标阶
                for j in (class + 1..i + 1).rev() {
                    if let Some(block) = self.free_list[j].pop() {
                        unsafe {
                            // 将大块拆分为两半，分别加入 j-1 阶的链表
                            self.free_list[j - 1]
                                .push((block as usize + (1 << (j - 1))) as *mut usize);
                            self.free_list[j - 1].push(block);
                        }
                    } else {
                        // 拆分过程中若遇空则分配失败
                        return Err(());
                    }
                }

                // 弹出目标阶的块作为返回结果
                let result = NonNull::new(
                    self.free_list[class]
                        .pop()
                        .expect("current block should have free space now")
                        as *mut u8,
                );
                if let Some(result) = result {
                    // 更新统计信息：用户请求字节数与实际分配字节数
                    self.user += layout.size();
                    self.allocated += size;
                    return Ok(result);
                } else {
                    return Err(());
                }
            }
        }
        // 所有阶均无可用块，返回分配失败
        Err(())
    }

    /// 释放一段内存回堆
    #[allow(unused)]
    pub fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        // 计算释放块的实际大小（满足对齐和 2 的幂要求）
        let size = max(
            layout.size().next_power_of_two(),
            max(layout.align(), size_of::<usize>()),
        );
        // 计算对应的阶次 class = log2(size)
        let class = size.trailing_zeros() as usize;

        unsafe {
            // 将释放的块地址放回对应阶的空闲链表
            self.free_list[class].push(ptr.as_ptr() as *mut usize);

            // 尝试与相邻伙伴块合并
            let mut current_ptr = ptr.as_ptr() as usize;
            let mut current_class = class;

            // 循环直至最高阶或不能继续合并
            while current_class < self.free_list.len() - 1 {
                // 计算当前块的伙伴地址
                let buddy = current_ptr ^ (1 << current_class);
                let mut found = false;
                // 遍历当前阶链表，查找空闲的伙伴块
                for block in self.free_list[current_class].iter_mut() {
                    if block.value() as usize == buddy {
                        block.pop(); // 移除伙伴块
                        found = true;
                        break;
                    }
                }

                // 如果找到伙伴块，则合并成更高阶块
                if found {
                    // 从链表中移除当前块
                    self.free_list[current_class].pop();
                    // 更新起始地址为合并后较小的地址
                    current_ptr = min(current_ptr, buddy);
                    // 升阶
                    current_class += 1;
                    // 将合并后的块插入更高阶链表
                    self.free_list[current_class].push(current_ptr as *mut usize);
                } else {
                    // 未找到伙伴则退出合并
                    break;
                }
            }
        }

        // 更新统计：减少用户请求字节和实际分配字节
        self.user -= layout.size();
        self.allocated -= size;
    }

    /// 返回用户实际请求的字节数
    #[allow(unused)]
    pub fn stats_alloc_user(&self) -> usize {
        self.user
    }

    /// 返回实际分配的字节数
    #[allow(unused)]
    pub fn stats_alloc_actual(&self) -> usize {
        self.allocated
    }

    /// 返回堆的总字节数
    #[allow(unused)]
    pub fn stats_total_bytes(&self) -> usize {
        self.total
    }
}

impl<const ORDER: usize> fmt::Debug for Heap<ORDER> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Heap {{ user: {}, allocated: {}, total: {}",
            self.user, self.allocated, self.total
        )?;
        for (i, list) in self.free_list.iter().enumerate() {
            write!(f, ",\n  free_list[{}]: [", i)?;
            let mut first = true;
            for node in list.iter() {
                if !first {
                    write!(f, ", ")?;
                }
                write!(f, "0x{:x}", node as usize)?;
                first = false;
            }
            write!(f, "]")?;
        }
        write!(f, " }}")
    }
}

/// 计算小于等于 num 的最大 2 的幂
pub(crate) fn prev_power_of_two(num: usize) -> usize {
    1 << (usize::BITS as usize - num.leading_zeros() as usize - 1)
}
