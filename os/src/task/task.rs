//! 与任务管理相关的类型
//!
//! 本模块实现了任务控制块（TCB）和任务状态管理，包括：
//! - 任务控制块结构定义和方法
//! - 任务状态枚举
//! - 从ELF文件创建任务
//! - 任务内存管理和堆管理

use super::TaskContext;
use crate::config::TRAP_CONTEXT_BASE;
use crate::console::color;
use crate::mm::debug::print_area_mapping;
use crate::mm::{
    kernel_stack_position, MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE,
};
use crate::trap::{trap_handler, TrapContext};

/// 任务控制块（TCB）
///
/// 包含管理一个任务/进程所需的所有信息，包括：
/// - 任务上下文：用于任务切换
/// - 任务状态：指示任务当前的运行状态
/// - 内存集合：任务的地址空间
/// - 陷阱上下文：处理中断和异常
/// - 程序相关信息：如大小、堆管理等

pub struct TaskControlBlock {
    /// 保存任务上下文
    /// 在任务切换时使用，包含寄存器状态
    pub task_cx: TaskContext,

    /// 维护当前进程的执行状态
    /// 可能是未初始化、就绪、运行或已退出
    pub task_status: TaskStatus,

    /// 应用程序地址空间
    /// 包含任务的内存映射、页表等
    pub memory_set: MemorySet,

    /// 陷阱上下文的物理页号
    /// 指向存储任务陷阱上下文的物理页
    pub trap_cx_ppn: PhysPageNum,

    /// 从 elf 文件加载的程序的大小（顶部地址）
    /// 通常对应于程序的数据段结束位置
    pub base_size: usize,

    /// 堆底部地址
    /// 程序堆内存分配的起始位置
    pub heap_bottom: usize,

    /// 程序中断点（program break）
    /// 表示当前堆的顶部，用于动态内存分配
    pub program_brk: usize,
}

impl TaskControlBlock {
    /// 获取陷阱上下文
    ///
    /// 通过物理页号访问任务的陷阱上下文
    ///
    /// # 返回值
    ///
    /// 返回指向陷阱上下文的可变引用

    pub fn get_trap_cx(&self) -> &'static mut TrapContext {

        // 使用物理页号获取页中存储的陷阱上下文
        self.trap_cx_ppn.get_mut()
    }

    /// 获取用户令牌
    ///
    /// 获取任务内存集中的页表令牌，用于设置satp寄存器
    ///
    /// # 返回值
    ///
    /// 返回可直接用于MMU的页表令牌

    pub fn get_user_token(&self) -> usize {

        // 从内存集中获取页表令牌
        self.memory_set.token()
    }

    /// 基于程序中的 elf 信息，在新的地址空间中构建任务控制块
    ///
    /// 从ELF文件创建新的任务，包括：
    /// 1. 创建任务的地址空间和内存映射
    /// 2. 设置用户栈和初始状态
    /// 3. 映射内核栈并创建上下文
    ///
    /// # 参数
    ///
    /// * `elf_data` - ELF文件的二进制数据
    /// * `app_id` - 应用程序ID，用于分配内核栈
    ///
    /// # 返回值
    ///
    /// 返回一个初始化的任务控制块

    pub fn new(elf_data: &[u8], app_id: usize) -> Self {

        // 从ELF文件创建内存集合，包含用户程序的各段、用户栈等
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);

        // 获取陷阱上下文的虚拟地址和物理页号
        let trap_context_base = TRAP_CONTEXT_BASE;

        let virt_page_num = VirtAddr::from(trap_context_base).into();

        let trap_cx_ppn = memory_set.translate(virt_page_num).unwrap().ppn();

        // 设置任务初始状态为就绪
        let task_status = TaskStatus::Ready;

        // 为任务分配内核栈，在内核空间中映射
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);

        {

            // 在内核空间中插入内核栈区域，权限为可读可写
            let start_va = kernel_stack_bottom.into();

            let end_va = kernel_stack_top.into();

            let permission = MapPermission::R | MapPermission::W;

            KERNEL_SPACE
                .exclusive_access()
                .insert_framed_area(start_va, end_va, permission);
        }

        // 打印内核栈映射信息
        print_area_mapping(
            "Kernel stack",
            &KERNEL_SPACE.exclusive_access().page_table,
            kernel_stack_bottom,
            kernel_stack_top,
            color::BRIGHT_YELLOW,
        );

        // 创建任务控制块结构
        let task_control_block = Self {
            task_status,
            // 创建任务上下文，返回地址设置为trap_return
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
            heap_bottom: user_sp,
            program_brk: user_sp,
        };

        // 设置陷阱上下文的初始值
        let trap_cx = task_control_block.get_trap_cx();

        *trap_cx = TrapContext::app_init_context(
            entry_point,                             // 程序入口点
            user_sp,                                 // 用户栈顶
            KERNEL_SPACE.exclusive_access().token(), // 内核页表令牌
            kernel_stack_top,                        // 内核栈顶
            trap_handler as usize,                   // 陷阱处理函数
        );

        task_control_block
    }

    /// 更改程序中断点的位置
    ///
    /// 调整任务的堆大小，可增大或缩小
    /// 实现类似 sbrk 系统调用的功能
    ///
    /// # 参数
    ///
    /// * `size` - 堆的调整大小，正数表示扩展，负数表示收缩
    ///
    /// # 返回值
    ///
    /// * `Some(usize)` - 成功则返回旧的break指针
    /// * `None` - 调整失败

    pub fn change_program_brk(&mut self, size: i32) -> Option<usize> {

        // 保存当前的program break
        let old_break = self.program_brk;

        // 计算新的program break
        let new_brk = self.program_brk as isize + size as isize;

        // 检查新的break是否小于堆底部，确保不会回收过多内存
        if new_brk < self.heap_bottom as isize {

            return None;
        }

        // 根据size的正负选择收缩或扩展内存区域
        let result = if size < 0 {

            // 收缩内存区域
            self.memory_set
                .shrink_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        } else {

            // 扩展内存区域
            self.memory_set
                .append_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        };

        // 如果内存调整成功，更新program_brk并返回旧值
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
///
/// 表示任务在生命周期中的不同阶段：
/// - UnInit: 任务已创建但尚未初始化
/// - Ready: 任务已初始化，可以运行
/// - Running: 任务正在运行中
/// - Exited: 任务已退出，资源可回收

pub enum TaskStatus {
    /// 未初始化
    /// 任务刚被创建，尚未准备好运行
    UnInit,
    /// 准备运行
    /// 任务已初始化并等待CPU时间
    Ready,
    /// 正在运行
    /// 任务当前正在CPU上执行
    Running,
    /// 已退出
    /// 任务已完成执行
    Exited,
}
