mod linked_list;
mod test;

use core::alloc::Layout;
use core::cmp::{max, min};
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
        // 避免某些平台上的未对齐访问
        start = (start + size_of::<usize>() - 1) & (!size_of::<usize>() + 1);
        end &= !size_of::<usize>() + 1;
        assert!(start <= end);

        let mut total = 0;
        let mut current_start = start;

        while current_start + size_of::<usize>() <= end {
            let lowbit = current_start & (!current_start + 1);
            let mut size = min(lowbit, prev_power_of_two(end - current_start));

            // 如果 size 的阶数大于最大阶数，则拆分为更小的块
            let mut order = size.trailing_zeros() as usize;
            if order > ORDER - 1 {
                order = ORDER - 1;
                size = 1 << order;
            }
            total += size;

            self.free_list[order].push(current_start as *mut usize);
            current_start += size;
        }

        self.total += total;
    }

    /// 向堆中添加一段内存区间 [start, start+size)
    #[allow(unused)]
    pub unsafe fn init(&mut self, start: usize, size: usize) {
        self.add_to_heap(start, start + size);
    }

    /// 从堆中分配一段满足 `layout` 要求的内存
    pub fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, ()> {
        let size = max(
            layout.size().next_power_of_two(),
            max(layout.align(), size_of::<usize>()),
        );
        let class = size.trailing_zeros() as usize;
        for i in class..self.free_list.len() {
            // 找到第一个非空的 size class
            if !self.free_list[i].is_empty() {
                // 拆分更大的块
                for j in (class + 1..i + 1).rev() {
                    if let Some(block) = self.free_list[j].pop() {
                        unsafe {
                            self.free_list[j - 1]
                                .push((block as usize + (1 << (j - 1))) as *mut usize);
                            self.free_list[j - 1].push(block);
                        }
                    } else {
                        return Err(());
                    }
                }

                let result = NonNull::new(
                    self.free_list[class]
                        .pop()
                        .expect("current block should have free space now")
                        as *mut u8,
                );
                if let Some(result) = result {
                    self.user += layout.size();
                    self.allocated += size;
                    return Ok(result);
                } else {
                    return Err(());
                }
            }
        }
        Err(())
    }

    /// 释放一段内存回堆
    pub fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        let size = max(
            layout.size().next_power_of_two(),
            max(layout.align(), size_of::<usize>()),
        );
        let class = size.trailing_zeros() as usize;

        unsafe {
            // 放回空闲链表
            self.free_list[class].push(ptr.as_ptr() as *mut usize);

            // 合并伙伴块
            let mut current_ptr = ptr.as_ptr() as usize;
            let mut current_class = class;

            while current_class < self.free_list.len() - 1 {
                let buddy = current_ptr ^ (1 << current_class);
                let mut flag = false;
                for block in self.free_list[current_class].iter_mut() {
                    if block.value() as usize == buddy {
                        block.pop();
                        flag = true;
                        break;
                    }
                }

                // 找到空闲伙伴块
                if flag {
                    self.free_list[current_class].pop();
                    current_ptr = min(current_ptr, buddy);
                    current_class += 1;
                    self.free_list[current_class].push(current_ptr as *mut usize);
                } else {
                    break;
                }
            }
        }

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

/// 计算小于等于 num 的最大 2 的幂
pub(crate) fn prev_power_of_two(num: usize) -> usize {
    1 << (usize::BITS as usize - num.leading_zeros() as usize - 1)
}
