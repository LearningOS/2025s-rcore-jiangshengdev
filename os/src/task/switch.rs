//! `__switch` 的 Rust 封装。
//!
//! 切换到不同任务的上下文发生在这里。实际的
//! 实现不能是 Rust，而（本质上）必须是汇编
//! 语言（你知道为什么吗？），所以这个模块实际上只是
//! `switch.S` 的一个封装。

core::arch::global_asm!(include_str!("switch.S"));

use super::TaskContext;

extern "C" {

    /// 切换到 `next_task_cx_ptr` 的上下文，将当前上下文
    /// 保存到 `current_task_cx_ptr` 中。
    pub fn __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext);

}
