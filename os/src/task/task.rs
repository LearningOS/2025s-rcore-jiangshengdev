//! 与任务管理相关的类型

use super::TaskContext;
use crate::config::TRAP_CONTEXT_BASE;
use crate::console::color;
use crate::mm::debug::print_area_mapping;
use crate::mm::{
    kernel_stack_position, MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE,
};
use crate::trap::{trap_handler, TrapContext};

/// 任务控制块（TCB）。

pub struct TaskControlBlock {
    /// 保存任务上下文
    pub task_cx: TaskContext,

    /// 维护当前进程的执行状态
    pub task_status: TaskStatus,

    /// 应用程序地址空间
    pub memory_set: MemorySet,

    /// 陷阱上下文的物理页号
    pub trap_cx_ppn: PhysPageNum,

    /// 从 elf 文件加载的程序的大小（顶部地址）
    pub base_size: usize,

    /// 堆底部
    pub heap_bottom: usize,

    /// 程序中断点
    pub program_brk: usize,
}

impl TaskControlBlock {
    /// 获取陷阱上下文

    pub fn get_trap_cx(&self) -> &'static mut TrapContext {

        self.trap_cx_ppn.get_mut()
    }

    /// 获取用户令牌

    pub fn get_user_token(&self) -> usize {

        self.memory_set.token()
    }

    /// 基于程序中的 elf 信息，在新的地址空间中构建任务内容

    pub fn new(elf_data: &[u8], app_id: usize) -> Self {

        // 带有 elf 程序头/跳板/陷阱上下文/用户栈的内存集
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);

        let trap_context_base = TRAP_CONTEXT_BASE;

        let virt_page_num = VirtAddr::from(trap_context_base).into();

        let trap_cx_ppn = memory_set.translate(virt_page_num).unwrap().ppn();

        let task_status = TaskStatus::Ready;

        // 在内核空间中映射一个内核栈
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);

        {

            let start_va = kernel_stack_bottom.into();

            let end_va = kernel_stack_top.into();

            let permission = MapPermission::R | MapPermission::W;

            KERNEL_SPACE
                .exclusive_access()
                .insert_framed_area(start_va, end_va, permission);
        }

        // 新增：打印内核栈映射信息，参考用户栈的打印
        print_area_mapping(
            "Kernel stack",
            &KERNEL_SPACE.exclusive_access().page_table,
            kernel_stack_bottom,
            kernel_stack_top,
            color::BRIGHT_YELLOW,
        );

        let task_control_block = Self {
            task_status,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
            heap_bottom: user_sp,
            program_brk: user_sp,
        };

        // 在用户空间准备陷阱上下文
        let trap_cx = task_control_block.get_trap_cx();

        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );

        task_control_block
    }

    /// 更改程序中断点的位置。如果失败则返回 None。

    pub fn change_program_brk(&mut self, size: i32) -> Option<usize> {

        let old_break = self.program_brk;

        let new_brk = self.program_brk as isize + size as isize;

        if new_brk < self.heap_bottom as isize {

            return None;
        }

        let result = if size < 0 {

            self.memory_set
                .shrink_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        } else {

            self.memory_set
                .append_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        };

        if result {

            self.program_brk = new_brk as usize;

            Some(old_break)
        } else {

            None
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
/// 任务状态：未初始化、就绪、运行、已退出

pub enum TaskStatus {
    /// 未初始化
    UnInit,
    /// 准备运行
    Ready,
    /// 正在运行
    Running,
    /// 已退出
    Exited,
}
