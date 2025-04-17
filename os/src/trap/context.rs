//! [`TrapContext`] 的实现。
use riscv::register::sstatus::{self, Sstatus, SPP};

#[repr(C)]
#[derive(Debug)]
/// trap 上下文结构体，包含 sstatus、sepc 及通用寄存器。
pub struct TrapContext {
    /// 通用寄存器 x0-31。
    pub x: [usize; 32],
    /// 管态状态寄存器。
    pub sstatus: Sstatus,
    /// 管态异常程序计数器。
    pub sepc: usize,
    /// 内核地址空间 token。
    pub kernel_satp: usize,
    /// 当前应用的内核栈指针。
    pub kernel_sp: usize,
    /// 内核中 trap 处理入口的虚拟地址。
    pub trap_handler: usize,
}

impl TrapContext {
    /// 将 sp（栈指针）写入 TrapContext 的 x[2] 字段。
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }
    /// 初始化应用的 trap 上下文。
    pub fn app_init_context(
        entry: usize,
        sp: usize,
        kernel_satp: usize,
        kernel_sp: usize,
        trap_handler: usize,
    ) -> Self {
        let mut sstatus = sstatus::read();
        // 设置 trap 返回后 CPU 特权级为 User。
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,  // 应用入口点
            kernel_satp,  // 页表地址
            kernel_sp,    // 内核栈
            trap_handler, // trap_handler 函数地址
        };
        cx.set_sp(sp); // 应用用户栈指针
        cx // 返回应用初始 Trap Context
    }
}
