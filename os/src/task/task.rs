//! 与任务管理相关的类型，以及用于完全切换 TCB 的函数

use super::TaskContext;
use super::{kstack_alloc, pid_alloc, KernelStack, PidHandle};
use crate::config::TRAP_CONTEXT_BASE;
use crate::mm::{MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE};
use crate::sync::UPSafeCell;
use crate::trap::{trap_handler, TrapContext};
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::cell::RefMut;

/// 任务控制块结构体
///
/// 直接保存运行期间不会改变的内容

pub struct TaskControlBlock {
    // 不可变部分
    /// 进程标识符
    pub pid: PidHandle,

    /// 与 PID 对应的内核栈
    pub kernel_stack: KernelStack,

    /// 可变部分
    inner: UPSafeCell<TaskControlBlockInner>,
}

impl TaskControlBlock {
    /// 获取内部 TCB 的可变引用
    ///
    /// # 返回值
    /// 返回对 TaskControlBlockInner 的独占可变引用。

    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner> {

        self.inner.exclusive_access()
    }

    /// 获取应用页表的地址
    ///
    /// # 返回值
    /// 返回当前进程用户空间页表的 token。

    pub fn get_user_token(&self) -> usize {

        let inner = self.inner_exclusive_access();

        inner.memory_set.token()
    }
}

pub struct TaskControlBlockInner {
    /// 放置 TrapContext 的物理页帧号
    pub trap_cx_ppn: PhysPageNum,

    /// 应用数据只能出现在应用地址空间低于 base_size 的区域
    pub base_size: usize,

    /// 保存任务上下文
    pub task_cx: TaskContext,

    /// 维护当前进程的执行状态
    pub task_status: TaskStatus,

    /// 应用地址空间
    pub memory_set: MemorySet,

    /// 当前进程的父进程。
    /// Weak 不会影响父进程的引用计数
    pub parent: Option<Weak<TaskControlBlock>>,

    /// 包含当前进程所有子进程 TCB 的向量
    pub children: Vec<Arc<TaskControlBlock>>,

    /// 主动退出或执行出错时设置
    pub exit_code: i32,

    /// 堆底
    pub heap_bottom: usize,

    /// 程序 break
    pub program_brk: usize,
}

impl TaskControlBlockInner {
    /// 获取 TrapContext
    ///
    /// # 返回值
    /// 返回指向 TrapContext 的可变静态引用。

    pub fn get_trap_cx(&self) -> &'static mut TrapContext {

        self.trap_cx_ppn.get_mut()
    }

    /// 获取用户 token
    ///
    /// # 返回值
    /// 返回当前进程用户空间页表的 token。

    pub fn get_user_token(&self) -> usize {

        self.memory_set.token()
    }

    /// 获取任务状态
    ///
    /// # 返回值
    /// 返回当前任务的状态。

    fn get_status(&self) -> TaskStatus {

        self.task_status
    }

    /// 判断是否为僵尸进程
    ///
    /// # 返回值
    /// 如果当前任务为僵尸进程，返回 true，否则返回 false。

    pub fn is_zombie(&self) -> bool {

        self.get_status() == TaskStatus::Zombie
    }
}

impl TaskControlBlock {
    /// 创建新进程
    ///
    /// 目前仅用于创建 initproc
    ///
    /// # 参数
    /// * `elf_data` - ELF 格式的应用程序二进制数据切片。
    ///
    /// # 返回值
    /// 返回新建的 TaskControlBlock。

    pub fn new(elf_data: &[u8]) -> Self {

        // memory_set 包含 elf 程序头／trampoline／trap context／用户栈
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);

        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT_BASE).into())
            .unwrap()
            .ppn();

        // 在内核空间分配 pid 和内核栈
        let pid_handle = pid_alloc();

        let kernel_stack = kstack_alloc();

        let kernel_stack_top = kernel_stack.get_top();

        // 将进入 trap_return 的任务上下文压入内核栈顶
        let task_control_block = Self {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {

                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: user_sp,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                    heap_bottom: user_sp,
                    program_brk: user_sp,
                })
            },
        };

        // 在用户空间准备 TrapContext
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();

        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );

        task_control_block
    }

    /// 加载新的 elf，替换原有应用地址空间并开始执行
    ///
    /// # 参数
    /// * `elf_data` - 新的 ELF 格式应用程序二进制数据切片。

    pub fn exec(&self, elf_data: &[u8]) {

        // memory_set 包含 elf 程序头／trampoline／trap context／用户栈
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);

        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT_BASE).into())
            .unwrap()
            .ppn();

        // **** 独占访问当前 TCB
        let mut inner = self.inner_exclusive_access();

        // 替换 memory_set
        inner.memory_set = memory_set;

        // 更新 trap_cx ppn
        inner.trap_cx_ppn = trap_cx_ppn;

        // 初始化 base_size
        inner.base_size = user_sp;

        // 初始化 trap_cx
        let trap_cx = inner.get_trap_cx();

        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            self.kernel_stack.get_top(),
            trap_handler as usize,
        );
        // **** 自动释放 inner
    }

    /// 父进程 fork 子进程
    ///
    /// # 返回值
    /// 返回新建的子进程 TaskControlBlock 的 Arc 智能指针。

    pub fn fork(self: &Arc<Self>) -> Arc<Self> {

        // ---- 独占访问父 PCB
        let mut parent_inner = self.inner_exclusive_access();

        // 拷贝用户空间（包括 trap context）
        let memory_set = MemorySet::from_existed_user(&parent_inner.memory_set);

        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT_BASE).into())
            .unwrap()
            .ppn();

        // 在内核空间分配 pid 和内核栈
        let pid_handle = pid_alloc();

        let kernel_stack = kstack_alloc();

        let kernel_stack_top = kernel_stack.get_top();

        let task_control_block = Arc::new(TaskControlBlock {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {

                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: parent_inner.base_size,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    heap_bottom: parent_inner.heap_bottom,
                    program_brk: parent_inner.program_brk,
                })
            },
        });

        // 添加子进程
        parent_inner.children.push(task_control_block.clone());

        // 修改 trap_cx 中的 kernel_sp
        // **** 独占访问子 PCB
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();

        trap_cx.kernel_sp = kernel_stack_top;

        // 返回
        task_control_block
        // **** 释放子 PCB
        // ---- 释放父 PCB
    }

    /// 获取进程的 pid
    ///
    /// # 返回值
    /// 返回当前进程的 pid。

    pub fn getpid(&self) -> usize {

        self.pid.0
    }

    /// 改变程序 break 的位置，失败返回 None
    ///
    /// # 参数
    /// * `size` - 需要调整的字节数，正数为扩展，负数为收缩。
    ///
    /// # 返回值
    /// 成功时返回原 program_brk，失败返回 None。

    pub fn change_program_brk(&self, size: i32) -> Option<usize> {

        let mut inner = self.inner_exclusive_access();

        let heap_bottom = inner.heap_bottom;

        let old_break = inner.program_brk;

        let new_brk = inner.program_brk as isize + size as isize;

        if new_brk < heap_bottom as isize {

            return None;
        }

        let result = if size < 0 {

            inner
                .memory_set
                .shrink_to(VirtAddr(heap_bottom), VirtAddr(new_brk as usize))
        } else {

            inner
                .memory_set
                .append_to(VirtAddr(heap_bottom), VirtAddr(new_brk as usize))
        };

        if result {

            inner.program_brk = new_brk as usize;

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
    /// 就绪
    Ready,
    /// 运行中
    Running,
    /// 已退出
    Zombie,
}
