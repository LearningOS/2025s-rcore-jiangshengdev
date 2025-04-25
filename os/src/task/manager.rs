//! [`TaskManager`] 的实现。

use super::TaskControlBlock;
use crate::console::named_color;
use crate::sync::UPSafeCell;
use alloc::collections::{BTreeMap, VecDeque};
use alloc::format;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;

// 程序名到背景RGB颜色的静态映射
fn app_name_color_map() -> &'static BTreeMap<&'static str, (u8, u8, u8)> {

    lazy_static! {
        static ref MAP: BTreeMap<&'static str, (u8, u8, u8)> = {

            let mut m = BTreeMap::new();

            m.insert("ch2b_bad_address", named_color::CRIMSON);

            m.insert("ch2b_bad_instructions", named_color::ORANGE);

            m.insert("ch2b_bad_register", named_color::GOLD);

            m.insert("ch2b_hello_world", named_color::SKYBLUE);

            m.insert("ch2b_power_3", named_color::LIMEGREEN);

            m.insert("ch2b_power_5", named_color::MEDIUMORCHID);

            m.insert("ch2b_power_7", named_color::CORNFLOWERBLUE);

            m.insert("ch3b_yield0", named_color::TOMATO);

            m.insert("ch3b_yield1", named_color::YELLOWGREEN);

            m.insert("ch3b_yield2", named_color::LIGHTPINK);

            m.insert("ch4b_sbrk", named_color::LIGHTSEAGREEN);

            m.insert("ch5b_exit", named_color::SADDLEBROWN);

            m.insert("ch5b_forktest", named_color::VIOLET);

            m.insert("ch5b_forktest2", named_color::LIGHTSTEELBLUE);

            m.insert("ch5b_forktest_simple", named_color::KHAKI);

            m.insert("ch5b_forktree", named_color::DARKORANGE);

            m.insert("ch5b_initproc", named_color::AQUAMARINE);

            m.insert("ch5b_user_shell", named_color::LAVENDER);

            m.insert("ch5b_usertest", named_color::PALEGREEN);

            m
        };
    }

    &MAP
}

/// 线程安全的 `TaskControlBlock` 队列。

pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// 简单的 FIFO 调度器。

impl Default for TaskManager {
    fn default() -> Self {

        Self::new()
    }
}

impl TaskManager {
    /// 创建一个空的 TaskManager。
    ///
    /// # 返回值
    /// 新的 TaskManager。

    pub fn new() -> Self {

        Self {
            ready_queue: VecDeque::new(),
        }
    }

    /// 将进程加入就绪队列。
    ///
    /// # 参数
    /// * `task` - 要加入的任务。

    pub fn add(&mut self, task: Arc<TaskControlBlock>) {

        println!();

        print_color!(crate::console::color::YELLOW, "[Add] Before enqueue:\n");

        self.print_queue_names();

        println!();

        let inner = task.inner_exclusive_access();

        let name = inner.name.clone();

        let pid = task.pid.0;

        drop(inner);

        print_color!(crate::console::color::GREEN, "[Add] Enqueue: ");

        print_task_brief(pid, &name);

        println!();

        self.ready_queue.push_back(task);

        print_color!(crate::console::color::CYAN, "[Add] After enqueue:\n");

        self.print_queue_names();
    }

    /// 从就绪队列取出一个进程。
    ///
    /// # 返回值
    /// 取出的任务（可选）。

    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {

        println!();

        print_color!(crate::console::color::YELLOW, "[Fetch] Before dequeue:\n");

        self.print_queue_names();

        println!();

        let task = self.ready_queue.pop_front();

        if let Some(ref t) = task {

            let inner = t.inner_exclusive_access();

            let name = inner.name.clone();

            let pid = t.pid.0;

            drop(inner);

            print_color!(crate::console::color::RED, "[Fetch] Dequeue: ");

            print_task_brief(pid, &name);

            println!();
        } else {

            println!("[Fetch] Dequeue: None");
        }

        print_color!(crate::console::color::CYAN, "[Fetch] After dequeue:\n");

        self.print_queue_names();

        task
    }

    /// 打印当前就绪队列中所有任务的 name

    pub fn print_queue_names(&self) {

        // 打印 (pid, name) 列表
        let infos: Vec<_> = self
            .ready_queue
            .iter()
            .map(|task| {

                let pid = task.pid.0;

                let name = task.inner_exclusive_access().name.clone();

                (pid, name)
            })
            .collect();

        let color_map = app_name_color_map();

        // Head and tail symbols and colors (English, no Chinese)
        let head_label = "HEAD";

        let tail_label = "TAIL";

        let head_color = crate::console::color::BRIGHT_GREEN;

        let tail_color = crate::console::color::BRIGHT_RED;

        let arrow = " <- ";

        let arrow_color = crate::console::color::BRIGHT_MAGENTA;

        print_color!(head_color, "{}", head_label);

        for (pid, name) in &infos {

            print_color!(arrow_color, "{}", arrow);

            let base_name = name.trim_end_matches('$');

            let pid_str = if *pid == usize::MAX {

                "-1".to_string()
            } else {

                pid.to_string()
            };

            if let Some(&color) = color_map.get(base_name) {

                if name.contains('$') {

                    let (r, g, b) = color;

                    let (r, g, b) = (r / 2, g / 2, b / 2);

                    print_bg_rgb!((r, g, b), "[{}] {}", pid_str, name);
                } else {

                    print_bg_named_rgb!(color, "[{}] {}", pid_str, name);
                }
            } else {

                print_color!(crate::console::color::RESET, "[{}] {}", pid_str, name);
            }
        }

        print_color!(arrow_color, "{}", arrow);

        print_color!(tail_color, "{}\n", tail_label);
    }
}

/// 创建并返回 TaskManager 的 UPSafeCell 实例。

fn create_task_manager() -> UPSafeCell<TaskManager> {

    unsafe {

        UPSafeCell::new(TaskManager::new())
    }
}

lazy_static! {
    /// 通过 lazy_static! 创建的 TASK_MANAGER 实例。
    #[no_mangle]
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> = create_task_manager();
}

/// 将进程加入就绪队列。
///
/// # 参数
/// * `task` - 要加入的任务。

pub fn add_task(task: Arc<TaskControlBlock>) {

    //trace!("kernel: TaskManager::add_task");
    let mut task_manager = TASK_MANAGER.exclusive_access();

    task_manager.add(task);
}

/// 从就绪队列取出一个进程。
///
/// # 返回值
/// 取出的任务（可选）。

pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {

    //trace!("kernel: TaskManager::fetch_task");
    let mut task_manager = TASK_MANAGER.exclusive_access();

    task_manager.fetch()
}

/// 按统一风格打印单个任务的 [pid] name，带背景色

pub fn print_task_brief(pid: usize, name: &str) {

    let color_map = app_name_color_map();

    let base_name = name.trim_end_matches('$');

    let pid_str = if pid == usize::MAX {

        "-1"
    } else {

        // 避免分配，直接格式化
        // 这里用 format! 兼容性更好
        &format!("{}", pid)
    };

    if let Some(&color) = color_map.get(base_name) {

        if name.contains('$') {

            let (r, g, b) = color;

            let (r, g, b) = (r / 2, g / 2, b / 2);

            print_bg_rgb!((r, g, b), "[{}] {}", pid_str, name);
        } else {

            print_bg_named_rgb!(color, "[{}] {}", pid_str, name);
        }
    } else {

        print_color!(crate::console::color::RESET, "[{}] {}", pid_str, name);
    }
}
