//! 任务管理实现
//!
//! 关于任务管理的所有内容，如启动和切换任务，均在此实现。
//!
//! 一个名为 `TASK_MANAGER` 的 [`TaskManager`] 全局实例控制
//! 操作系统中的所有任务。
//!
//! 在 `switch.S` 中当你看到 `__switch` 汇编函数时要小心。该函数
//! 周围的控制流可能与你预期的不同。

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

/// 任务管理器，用于管理所有任务。
///
/// 在 `TaskManager` 上实现的函数处理所有任务状态转换
/// 和任务上下文切换。为方便起见，你可以在模块级别
/// 中找到围绕它的包装器。
///
/// `TaskManager` 的大部分内容隐藏在字段 `inner` 后面，
/// 以将借用检查推迟到运行时。你可以在 `TaskManager`
/// 上已有的函数中看到如何使用 `inner` 的示例。

pub struct TaskManager {
    /// 任务总数
    num_app: usize,
    /// 使用内部值获取可变访问权限
    inner: UPSafeCell<TaskManagerInner>,
}

/// 'UPSafeCell' 中的任务管理器内部结构

struct TaskManagerInner {
    /// 任务列表
    tasks: Vec<TaskControlBlock>,
    /// 当前 `Running` 任务的 id
    current_task: usize,
}

lazy_static! {
    /// 通过 lazy_static! 创建的 `TaskManager` 全局实例
    pub static ref TASK_MANAGER: TaskManager = {
        println!("初始化 TASK_MANAGER");
        let num_app = get_num_app();
        println!("num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            println!("应用程序: {}", i);
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }
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
    /// 运行任务列表中的第一个任务。
    ///
    /// 通常，任务列表中的第一个任务是一个空闲任务（我们稍后称之为零进程）。
    /// 但在第 4 章中，我们静态加载应用程序，所以第一个任务是一个实际的应用程序。

    fn run_first_task(&self) -> ! {

        let mut inner = self.inner.exclusive_access();

        let next_task = &mut inner.tasks[0];

        next_task.task_status = TaskStatus::Running;

        let next_task_cx_ptr = &next_task.task_cx as *const TaskContext;

        drop(inner);

        let mut _unused = TaskContext::zero_init();

        // 在这之前，我们应该释放必须手动释放的局部变量
        unsafe {

            __switch(&mut _unused as *mut _, next_task_cx_ptr);
        }

        panic!("run_first_task 中不可达！");
    }

    /// 将当前 `Running` 任务的状态更改为 `Ready`。

    fn mark_current_suspended(&self) {

        let mut inner = self.inner.exclusive_access();

        let cur = inner.current_task;

        inner.tasks[cur].task_status = TaskStatus::Ready;
    }

    /// 将当前 `Running` 任务的状态更改为 `Exited`。

    fn mark_current_exited(&self) {

        let mut inner = self.inner.exclusive_access();

        let cur = inner.current_task;

        inner.tasks[cur].task_status = TaskStatus::Exited;
    }

    /// 查找下一个要运行的任务并返回任务 id。
    ///
    /// 在这种情况下，我们只返回任务列表中第一个处于 `Ready` 状态的任务。

    fn find_next_task(&self) -> Option<usize> {

        let inner = self.inner.exclusive_access();

        let current = inner.current_task;

        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// 获取当前 'Running' 任务的令牌。

    fn get_current_token(&self) -> usize {

        let inner = self.inner.exclusive_access();

        inner.tasks[inner.current_task].get_user_token()
    }

    /// 获取当前 'Running' 任务的陷阱上下文。

    fn get_current_trap_cx(&self) -> &'static mut TrapContext {

        let inner = self.inner.exclusive_access();

        inner.tasks[inner.current_task].get_trap_cx()
    }

    /// 更改当前 'Running' 任务的程序中断点

    pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {

        let mut inner = self.inner.exclusive_access();

        let cur = inner.current_task;

        inner.tasks[cur].change_program_brk(size)
    }

    /// 将当前 `Running` 任务切换到我们找到的任务，
    /// 或者如果没有 `Ready` 任务，我们可以在所有应用程序完成后退出

    fn run_next_task(&self) {

        if let Some(next) = self.find_next_task() {

            let mut inner = self.inner.exclusive_access();

            let current = inner.current_task;

            inner.tasks[next].task_status = TaskStatus::Running;

            inner.current_task = next;

            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;

            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;

            drop(inner);

            // 在这之前，我们应该释放必须手动释放的局部变量
            unsafe {

                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // 返回用户态
        } else {

            panic!("所有应用程序已完成！");
        }
    }
}

/// 运行任务列表中的第一个任务。

pub fn run_first_task() {

    TASK_MANAGER.run_first_task();
}

/// 将当前 `Running` 任务切换到我们找到的任务，
/// 或者如果没有 `Ready` 任务，我们可以在所有应用程序完成后退出

fn run_next_task() {

    TASK_MANAGER.run_next_task();
}

/// 将当前 `Running` 任务的状态更改为 `Ready`。

fn mark_current_suspended() {

    TASK_MANAGER.mark_current_suspended();
}

/// 将当前 `Running` 任务的状态更改为 `Exited`。

fn mark_current_exited() {

    TASK_MANAGER.mark_current_exited();
}

/// 挂起当前 'Running' 任务并运行任务列表中的下一个任务。

pub fn suspend_current_and_run_next() {

    mark_current_suspended();

    run_next_task();
}

/// 退出当前 'Running' 任务并运行任务列表中的下一个任务。

pub fn exit_current_and_run_next() {

    mark_current_exited();

    run_next_task();
}

/// 获取当前 'Running' 任务的令牌。

pub fn current_user_token() -> usize {

    TASK_MANAGER.get_current_token()
}

/// 获取当前 'Running' 任务的陷阱上下文。

pub fn current_trap_cx() -> &'static mut TrapContext {

    TASK_MANAGER.get_current_trap_cx()
}

/// 更改当前 'Running' 任务的程序中断点

pub fn change_program_brk(size: i32) -> Option<usize> {

    TASK_MANAGER.change_current_program_brk(size)
}
