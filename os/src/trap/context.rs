//! [`TrapContext`] 的实现

use riscv::register::sstatus::{self, Sstatus, SPP};

#[repr(C)]
#[derive(Debug)]
/// 包含 sstatus、sepc 和寄存器的陷阱上下文结构

pub struct TrapContext {
    /// 通用寄存器 x0-31
    pub x: [usize; 32],
    /// 监督者状态寄存器
    pub sstatus: Sstatus,
    /// 监督者异常程序计数器
    pub sepc: usize,
    /// 内核地址空间的令牌
    pub kernel_satp: usize,
    /// 当前应用程序的内核栈指针
    pub kernel_sp: usize,
    /// 内核中陷阱处理入口点的虚拟地址
    pub trap_handler: usize,
}

impl TrapContext {
    /// 将 sp（栈指针）放入 TrapContext 的 x\[2\] 字段

    pub fn set_sp(&mut self, sp: usize) {

        self.x[2] = sp;
    }

    /// 初始化应用程序的陷阱上下文

    pub fn app_init_context(
        entry: usize,
        sp: usize,
        kernel_satp: usize,
        kernel_sp: usize,
        trap_handler: usize,
    ) -> Self {

        let mut sstatus = sstatus::read();

        // 陷阱返回后将 CPU 权限设置为用户态
        sstatus.set_spp(SPP::User);

        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,  // 应用程序的入口点
            kernel_satp,  // 页表地址
            kernel_sp,    // 内核栈
            trap_handler, // 陷阱处理函数的地址
        };

        cx.set_sp(sp); // 应用程序的用户栈指针
        cx // 返回应用程序的初始陷阱上下文
    }
}
