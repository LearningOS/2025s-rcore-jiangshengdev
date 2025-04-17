//! [`TaskContext`] 的实现。
use crate::trap::trap_return;

#[repr(C)]
/// 任务上下文结构体，包含部分寄存器。
pub struct TaskContext {
    /// 任务切换后返回地址。
    ra: usize,
    /// 栈指针。
    sp: usize,
    /// s0-11 寄存器，调用者保存。
    s: [usize; 12],
}

impl TaskContext {
    /// 创建一个全零的任务上下文。
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }
    /// 创建一个带 trap 返回地址和内核栈指针的任务上下文。
    pub fn goto_trap_return(kstack_ptr: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}
