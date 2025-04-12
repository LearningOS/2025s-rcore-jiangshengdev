//! 任务管理实现
//!
//! 本模块实现操作系统的多任务管理，负责任务的创建、调度和切换，包括：
//! - 任务控制块（TCB）结构的定义与管理
//! - 任务调度器的实现
//! - 任务状态转换与上下文切换机制
//! - 内核栈的分配与管理
//!
//! 一个名为 `TASK_MANAGER` 的 [`TaskManager`] 全局实例控制
//! 操作系统中的所有任务，提供统一的任务管理接口。
//!
//! 在 `switch.S` 中的 `__switch` 汇编函数实现了底层的上下文切换操作。

mod context;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::loader::{get_app_data, get_num_app};
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::vec::Vec;
use lazy_static::*;
use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;

/// 任务管理器，用于管理所有任务
///
/// 任务管理器是操作系统任务调度的核心组件，负责：
/// 1. 维护系统中所有任务的列表和状态
/// 2. 实现任务的创建、切换、挂起和退出等操作
/// 3. 提供任务调度算法，决定下一个执行的任务
/// 4. 管理任务上下文和状态转换
///
/// 任务管理器的大部分实现隐藏在 `inner` 字段中，通过 UPSafeCell 实现内部可变性，
/// 使得在单处理器环境下能安全地访问和修改任务状态。

pub struct TaskManager {
    /// 任务总数
    /// 记录系统中应用程序的总数量
    num_app: usize,

    /// 使用内部可变性获取可变访问权限
    /// 包含任务列表和当前正在运行的任务ID
    inner: UPSafeCell<TaskManagerInner>,
}

/// 任务管理器内部结构
///
/// 封装在 `UPSafeCell` 中以实现内部可变性，包含：
/// - 所有任务的控制块列表
/// - 当前正在运行的任务ID

struct TaskManagerInner {
    /// 任务列表
    /// 存储系统中所有任务的控制块
    tasks: Vec<TaskControlBlock>,

    /// 当前 `Running` 任务的 id
    /// 指示当前正在CPU上执行的任务索引
    current_task: usize,
}

lazy_static! {
    /// 任务管理器全局实例
    ///
    /// 通过 lazy_static! 延迟初始化，在首次使用时创建
    /// 初始化过程包括：
    /// 1. 获取应用程序数量和数据
    /// 2. 为每个应用程序创建任务控制块
    /// 3. 创建并返回任务管理器实例
    pub static ref TASK_MANAGER: TaskManager = {
        println!("初始化 TASK_MANAGER");
        // 获取应用程序总数
        let num_app = get_num_app();
        println!("num_app = {}", num_app);

        // 为每个应用程序创建任务控制块
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            println!("应用程序: {}", i);
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }

        // 创建任务管理器
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

impl TaskManager {
    /// 运行任务列表中的第一个任务
    ///
    /// 初始化任务调度环境并启动第一个任务：
    /// 1. 获取任务列表中的第一个任务
    /// 2. 将其状态设置为Running
    /// 3. 获取其任务上下文指针
    /// 4. 调用__switch切换到该任务
    ///
    /// 该函数应在系统启动时调用，用于启动第一个用户程序
    ///
    /// # 注意
    ///
    /// 这个函数不会返回，它会直接切换到第一个任务的执行流

    fn run_first_task(&self) -> ! {

        // 获取内部数据的可变引用
        let mut inner = self.inner.exclusive_access();

        // 获取第一个任务并将其状态设置为Running
        let next_task = &mut inner.tasks[0];

        next_task.task_status = TaskStatus::Running;

        // 获取下一个任务的上下文指针
        let next_task_cx_ptr = &next_task.task_cx as *const TaskContext;

        // 释放内部数据锁，避免死锁
        drop(inner);

        // 创建一个临时的空上下文
        let mut _unused = TaskContext::zero_init();

        // 切换到第一个任务
        // 这是一个不安全的操作，因为它直接操作CPU寄存器并切换执行流
        unsafe {

            __switch(&mut _unused as *mut _, next_task_cx_ptr);
        }

        // 这里永远不会被执行到，因为__switch不会返回到这里
        panic!("run_first_task 中不可达！");
    }

    /// 将当前运行任务的状态标记为挂起(Ready)
    ///
    /// 获取当前正在运行的任务，并将其状态从Running更改为Ready
    /// 在任务主动让出CPU时使用

    fn mark_current_suspended(&self) {

        // 获取内部数据
        let mut inner = self.inner.exclusive_access();

        // 获取当前任务索引
        let cur = inner.current_task;

        // 将当前任务状态设置为就绪(Ready)
        inner.tasks[cur].task_status = TaskStatus::Ready;
    }

    /// 将当前运行任务的状态标记为已退出(Exited)
    ///
    /// 获取当前正在运行的任务，并将其状态从Running更改为Exited
    /// 在任务执行完成时使用

    fn mark_current_exited(&self) {

        // 获取内部数据
        let mut inner = self.inner.exclusive_access();

        // 获取当前任务索引
        let cur = inner.current_task;

        // 将当前任务状态设置为已退出(Exited)
        inner.tasks[cur].task_status = TaskStatus::Exited;
    }

    /// 查找下一个就绪的任务并返回其ID
    ///
    /// 采用简单的轮转调度算法(Round Robin)：
    /// 从当前任务的下一个开始遍历所有任务，
    /// 返回第一个状态为Ready的任务ID
    ///
    /// # 返回值
    ///
    /// * `Some(usize)` - 找到了下一个就绪的任务，返回其ID
    /// * `None` - 没有就绪的任务

    fn find_next_task(&self) -> Option<usize> {

        // 获取内部数据
        let inner = self.inner.exclusive_access();

        // 获取当前任务ID
        let current = inner.current_task;

        // 从当前任务ID+1开始，遍历一整圈(num_app个任务)
        // 使用模运算确保在任务数组范围内循环
        // 返回第一个状态为Ready的任务
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// 获取当前运行任务的用户空间令牌
    ///
    /// 返回当前任务的页表令牌，用于设置MMU进行地址转换
    ///
    /// # 返回值
    ///
    /// 当前任务的页表令牌，可用于satp寄存器

    fn get_current_token(&self) -> usize {

        // 获取内部数据
        let inner = self.inner.exclusive_access();

        // 返回当前任务的用户空间令牌
        inner.tasks[inner.current_task].get_user_token()
    }

    /// 获取当前运行任务的陷阱上下文
    ///
    /// 返回指向当前任务陷阱上下文的可变引用
    /// 用于在中断处理过程中访问和修改用户程序的上下文
    ///
    /// # 返回值
    ///
    /// 当前任务的陷阱上下文的静态可变引用

    fn get_current_trap_cx(&self) -> &'static mut TrapContext {

        // 获取内部数据
        let inner = self.inner.exclusive_access();

        // 获取当前任务的陷阱上下文
        inner.tasks[inner.current_task].get_trap_cx()
    }

    /// 更改当前运行任务的堆边界(program break)
    ///
    /// 实现类似sbrk系统调用的功能，调整当前任务的堆大小
    ///
    /// # 参数
    ///
    /// * `size` - 调整堆的大小(字节)，正数为扩大，负数为缩小
    ///
    /// # 返回值
    ///
    /// * `Some(usize)` - 成功调整堆大小，返回旧的program break
    /// * `None` - 调整失败

    pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {

        // 获取内部数据
        let mut inner = self.inner.exclusive_access();

        // 获取当前任务索引
        let cur = inner.current_task;

        // 调整当前任务的program break
        inner.tasks[cur].change_program_brk(size)
    }

    /// 切换到下一个就绪任务
    ///
    /// 主要步骤：
    /// 1. 查找下一个就绪的任务
    /// 2. 将该任务状态设置为Running
    /// 3. 更新当前任务ID
    /// 4. 保存当前上下文，并加载下一个任务的上下文
    /// 5. 使用__switch实现上下文切换
    ///
    /// 如果没有就绪任务，则触发panic

    fn run_next_task(&self) {

        // 查找下一个就绪任务
        if let Some(next) = self.find_next_task() {

            // 获取内部数据
            let mut inner = self.inner.exclusive_access();

            // 保存当前任务ID
            let current = inner.current_task;

            // 更新下一个任务的状态为Running
            inner.tasks[next].task_status = TaskStatus::Running;

            // 更新当前任务ID为下一个任务的ID
            inner.current_task = next;

            // 获取当前任务上下文指针和下一个任务上下文指针
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;

            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;

            // 释放内部数据锁，避免在上下文切换时持有锁
            drop(inner);

            // 进行上下文切换
            // 保存当前上下文到current_task_cx_ptr，并加载next_task_cx_ptr指向的上下文
            unsafe {

                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // 上下文切换完成后，会从用户态返回到这里继续执行
        } else {

            // 如果没有就绪任务，表示所有应用程序已完成
            panic!("所有应用程序已完成！");
        }
    }
}

/// 运行第一个任务
///
/// 调用 TASK_MANAGER 的 run_first_task 方法启动第一个任务
/// 这是操作系统进入用户空间前的最后一步
///
/// # 注意
///
/// 该函数不会返回，系统将进入多任务调度模式

pub fn run_first_task() {

    TASK_MANAGER.run_first_task();
}

/// 运行下一个就绪的任务
///
/// 调用 TASK_MANAGER 的 run_next_task 方法，切换到下一个就绪的任务
/// 如果没有就绪的任务，会导致系统panic

fn run_next_task() {

    TASK_MANAGER.run_next_task();
}

/// 标记当前任务为挂起状态
///
/// 调用 TASK_MANAGER 的 mark_current_suspended 方法
/// 将当前任务的状态从Running更改为Ready

fn mark_current_suspended() {

    TASK_MANAGER.mark_current_suspended();
}

/// 标记当前任务为退出状态
///
/// 调用 TASK_MANAGER 的 mark_current_exited 方法
/// 将当前任务的状态从Running更改为Exited

fn mark_current_exited() {

    TASK_MANAGER.mark_current_exited();
}

/// 挂起当前任务并运行下一个任务
///
/// 组合了mark_current_suspended和run_next_task操作
/// 用于当前任务主动让出CPU时（如sleep或yield系统调用）

pub fn suspend_current_and_run_next() {

    // 将当前任务标记为挂起状态
    mark_current_suspended();

    // 切换到下一个任务
    run_next_task();
}

/// 退出当前任务并运行下一个任务
///
/// 组合了mark_current_exited和run_next_task操作
/// 用于当前任务执行完成时（如exit系统调用）

pub fn exit_current_and_run_next() {

    // 将当前任务标记为退出状态
    mark_current_exited();

    // 切换到下一个任务
    run_next_task();
}

/// 获取当前任务的用户空间令牌
///
/// 返回当前正在运行的任务的页表令牌
/// 用于MMU地址转换过程中设置satp寄存器
///
/// # 返回值
///
/// 当前任务的页表令牌

pub fn current_user_token() -> usize {

    TASK_MANAGER.get_current_token()
}

/// 获取当前任务的陷阱上下文
///
/// 返回当前正在运行的任务的陷阱上下文引用
/// 用于系统调用和异常处理过程中访问和修改用户程序的上下文
///
/// # 返回值
///
/// 当前任务陷阱上下文的静态可变引用

pub fn current_trap_cx() -> &'static mut TrapContext {

    TASK_MANAGER.get_current_trap_cx()
}

/// 更改当前任务的堆大小
///
/// 调用 TASK_MANAGER 的 change_current_program_brk 方法
/// 用于实现 sbrk 系统调用的功能
///
/// # 参数
///
/// * `size` - 调整堆的大小(字节)，正数为扩大，负数为缩小
///
/// # 返回值
///
/// * `Some(usize)` - 成功调整堆大小，返回旧的program break
/// * `None` - 调整失败

pub fn change_program_brk(size: i32) -> Option<usize> {

    TASK_MANAGER.change_current_program_brk(size)
}
