### 功能实现总结

- 使用 Map 在 TCB 中记录系统调用次数
- syscall 函数最开始进行记录
- 调用 trace 时进行查询

### 遇到的问题

- 一开始用的数组直接存在 TCB 中，初始化时在栈上分配了太多空间，导致溢出卡住

### 简答作业

1. 正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。
   请同学们可以自行测试这些内容（运行
   `三个 bad 测例 (ch2b_bad_*.rs) <https://github.com/LearningOS/rCore-Tutorial-Test-2025S/tree/master/src/bin>`），
   描述程序出错行为，同时注意注明你使用的 sbi 及其版本。

- ch2b_bad_address.rs

  ```rust
  pub fn main() -> isize {
      unsafe {
          #[allow(clippy::zero_ptr)]
          (0x0 as *mut u8).write_volatile(0);
       }
  }
  ```

  - qemu-7.0.0/hw/riscv/virt.c

    ```c
    static const MemMapEntry virt_memmap[] = {
    [VIRT_DEBUG] =       {        0x0,         0x100 },
    [VIRT_DRAM] =        { 0x80000000,           0x0 },
    ```

    根据 virt 硬件可知：0x0 处非物理内存地址，而是 Debug 用的内存映射，不可写入

    写入触发了 Trap::Exception (Exception::StoreFault) 异常

- ch2b_bad_instructions.rs

  ```rust
  pub fn main() -> ! {
      unsafe {
          core::arch::asm!("sret");
      }
  }
  ```

  xRET 指令可以在特权模式 x 或更高模式下执行，不可在用户模式下执行

  触发了 Trap::Exception (Exception::IllegalInstruction)

- ch2b_bad_register.rs

  ```rust
  pub fn main() -> ! {
      let mut sstatus: usize;
      unsafe {
          core::arch::asm!("csrr {}, sstatus", out(reg) sstatus);
      }
  }
  ```

  特权级别用于在软件栈的不同组件之间提供保护，尝试执行当前特权模式不允许的操作将导致引发异常。这些异常通常会导致陷入底层执行环境。

  监督者状态 (sstatus) 寄存器是一个监督级 CSR，不可在用户模式下操作

  触发了 Trap::Exception (Exception::IllegalInstruction)

  ```
  [rustsbi] RustSBI version 0.3.0-alpha.2, adapting to RISC-V SBI v1.0.0
  [rustsbi] Implementation     : RustSBI-QEMU Version 0.2.0-alpha.2
  ```

2. 深入理解 `trap.S <https://github.com/LearningOS/rCore-Tutorial-Code-2025S/blob/ch3/os/src/trap/trap.S>`
   中两个函数 `__alltraps` 和 `__restore` 的作用，并回答如下问题:

   1. L40：刚进入 `__restore` 时，`sp` 代表了什么值。请指出 `__restore` 的两种使用情景。

   刚进入 \_\_restore 时，上一步在执行 switch，switch 中最后将内核栈地址写入了 sp 寄存器

   所以其代表了 **内核栈顶**，内核栈顶保存了 TrapContext 的信息

   - 一种是第一个程序 run_first_task 执行完 switch 之后

   - 另一种是 2 个程序切换 run_next_task 执行完 switch 之后

   2. L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。

      ```asm
      ld t0, 32*8(sp)
      ld t1, 33*8(sp)
      ld t2, 2*8(sp)
      csrw sstatus, t0
      csrw sepc, t1
      csrw sscratch, t2
      ```

   处理了 sstatus, sepc, sscratch 3 个寄存器

   - sstatus 当执行 SRET 指令以从陷阱处理程序返回时，如果 sstatus 寄存器的 SPP 位为 0，则特权级别设置为用户模式

   - sepc 暂存了上一次用户程序暂停的位置，是接下来用户程序要继续执行的 pc 位置

   - sscratch 暂存了用户上一次的 sp 栈位置，是接下来用户程序需要接着继续使用的

   3. L50-L56：为何跳过了 `x2` 和 `x4`？

      ```asm
      ld x1, 1*8(sp)
      ld x3, 3*8(sp)
      .set n, 5
      .rept 27
      LOAD_GP %n
      .set n, n+1
      .endr
      ```

   标准调用约定使用寄存器 x2 作为堆栈指针，x2 即是 sp 栈指针，暂时需要他来进行相对偏移操作不能覆盖，后续会单独处理
   x4 即是 tp 线程指针 (Thread pointer)，其值为当前的硬件线程编号，在系统的整个生命周期内保持不变，无需处理

   4. L60：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？

   ```asm
   csrrw sp, sscratch, sp
   ```

   - 执行之前，sp 为内核栈指针，sscratch 为用户栈指针

   - 执行之后，sp 为用户栈指针，sscratch 为内核栈指针

   5. `__restore`：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

   状态切换发生在 sret 指令，因为在此之前 sstatus 寄存器的 SPP 位被置为了 0

   6. L13：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？

      ```asm
      csrrw sp, sscratch, sp
      ```

   \_\_alltraps 中

   - 执行之前，sp 为用户栈指针，sscratch 为内核栈指针

   - 执行之后，sp 为内核栈指针，sscratch 为用户栈指针

   7. 从 U 态进入 S 态是哪一条指令发生的？

   - 有可能是由用户程序中的 ecall 指令造成的，其为用户态去要求执行系统调用
   - 还有可能是时钟造成。当系统使用 ecall 指令通知 SBI 设置时钟中断，待时间到期后，如果处于 U 态，也会进入到 S 态
