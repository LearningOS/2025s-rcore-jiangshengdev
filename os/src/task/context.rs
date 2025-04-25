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
    /// 任务名称。
    name: [u8; 24],
    /// 任务 pid。
    pid: usize,
}

impl TaskContext {
    /// 将字符串转为定长名称缓冲区

    fn make_name_buf(name: &str) -> [u8; 24] {

        let mut buf = [0u8; 24];

        let bytes = name.as_bytes();

        let len = bytes.len().min(buf.len() - 1);

        buf[..len].copy_from_slice(&bytes[..len]);

        buf
    }

    /// 创建一个全零的任务上下文。
    ///
    /// # 参数
    /// * `name` - 任务名称。
    /// * `pid` - 任务 pid。
    ///
    /// # 返回值
    /// 返回全零的 TaskContext。

    pub fn zero_init(name: &str, pid: usize) -> Self {

        let name_buf = Self::make_name_buf(name);

        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
            name: name_buf,
            pid,
        }
    }

    /// 创建一个带 trap 返回地址和内核栈指针的任务上下文。
    ///
    /// # 参数
    /// * `kstack_ptr` - 内核栈指针。
    /// * `name` - 任务名称。
    /// * `pid` - 任务 pid。
    ///
    /// # 返回值
    /// 返回新的 TaskContext。

    pub fn goto_trap_return(kstack_ptr: usize, name: &str, pid: usize) -> Self {

        let name_buf = Self::make_name_buf(name);

        Self {
            ra: trap_return as usize,
            sp: kstack_ptr,
            s: [0; 12],
            name: name_buf,
            pid,
        }
    }

    /// 获取任务名称。
    ///
    /// # 返回值
    /// 返回任务名称。

    pub fn debug_name(&self) -> &str {

        let nul_pos = self
            .name
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(self.name.len());

        core::str::from_utf8(&self.name[..nul_pos]).unwrap_or("")
    }

    /// 获取任务 pid。

    pub fn debug_pid(&self) -> usize {

        self.pid
    }

    /// 更新任务调度时显示的名称

    pub fn set_name(&mut self, name: &str) {

        self.name = Self::make_name_buf(name);
    }

    /// 更新任务调度时显示的名称和 pid

    pub fn set_name_pid(&mut self, name: &str, pid: usize) {

        self.name = Self::make_name_buf(name);

        self.pid = pid;
    }
}
