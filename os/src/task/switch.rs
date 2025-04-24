//! 将 `switch.S` 封装为函数

use super::TaskContext;
use core::arch::global_asm;

global_asm!(include_str!("switch.S"));

extern "C" {

    /// 切换到 `next_task_cx_ptr` 指向的上下文，并保存当前上下文到 `current_task_cx_ptr`。
    pub fn __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext);

}
