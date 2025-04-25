//! Print the fork tree of all processes

use super::{TaskControlBlock, TaskStatus, INITPROC};
use crate::console::color;
use crate::console::print_colorful;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

/// 打印进程树（fork tree），从 INITPROC 开始递归遍历
pub fn print_fork_tree() {
    fn print_tree_rec(task: &Arc<TaskControlBlock>, ancestors_last: &Vec<bool>) {
        let inner = task.inner_exclusive_access();
        let status = match inner.task_status {
            TaskStatus::UnInit => "UnInit",
            TaskStatus::Ready => "Ready",
            TaskStatus::Running => "Running",
            TaskStatus::Zombie => "Zombie",
        };
        // 根据状态选择颜色
        let col = match inner.task_status {
            TaskStatus::UnInit => color::YELLOW,
            TaskStatus::Ready => color::GREEN,
            TaskStatus::Running => color::CYAN,
            TaskStatus::Zombie => color::RED,
        };
        if ancestors_last.is_empty() {
            // 根节点整个信息都着色
            print_colorful(
                col,
                format_args!("[{}] {} [{}]\n", task.getpid(), inner.name, status),
            );
        } else {
            // 构造并打印前缀，前缀不着色
            let mut prefix = String::new();
            for &last in &ancestors_last[..ancestors_last.len() - 1] {
                prefix.push_str(if last { "    " } else { "│   " });
            }
            let branch = if *ancestors_last.last().unwrap() {
                "└── "
            } else {
                "├── "
            };
            prefix.push_str(branch);
            // 打印分支和整个节点信息
            print!("{}", prefix);
            print_colorful(
                col,
                format_args!("[{}] {} [{}]\n", task.getpid(), inner.name, status),
            );
        }
        // 遍历子节点
        let child_count = inner.children.len();
        for (i, child) in inner.children.iter().enumerate() {
            let mut new_ancestors = ancestors_last.clone();
            new_ancestors.push(i == child_count - 1);
            print_tree_rec(child, &new_ancestors);
        }
    }
    println!("Fork Tree:");
    print_tree_rec(&INITPROC, &Vec::new());
    println!();
}
