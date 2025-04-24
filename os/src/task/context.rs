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
}

impl TaskContext {
    /// 创建一个全零的任务上下文。
    ///
    /// # 参数
    /// * `name` - 任务名称。
    ///
    /// # 返回值
    /// 返回全零的 TaskContext。

    pub fn zero_init(name: &str) -> Self {

        let mut name_buf = [0u8; 24];

        let bytes = name.as_bytes();

        let len = bytes.len().min(23);

        name_buf[..len].copy_from_slice(&bytes[..len]);

        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
            name: name_buf,
        }
    }

    /// 创建一个带 trap 返回地址和内核栈指针的任务上下文。
    ///
    /// # 参数
    /// * `kstack_ptr` - 内核栈指针。
    /// * `name` - 任务名称。
    ///
    /// # 返回值
    /// 返回新的 TaskContext。

    pub fn goto_trap_return(kstack_ptr: usize, name: &str) -> Self {

        let mut name_buf = [0u8; 24];

        let bytes = name.as_bytes();

        let len = bytes.len().min(23);

        name_buf[..len].copy_from_slice(&bytes[..len]);

        Self {
            ra: trap_return as usize,
            sp: kstack_ptr,
            s: [0; 12],
            name: name_buf,
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
}
