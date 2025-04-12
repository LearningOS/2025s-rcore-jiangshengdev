.altmacro
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm
    .section .text.trampoline
    .globl __alltraps
    .globl __restore
    .align 2
__alltraps:
    csrrw sp, sscratch, sp
    # 现在 sp 指向用户空间中的 *TrapContext，sscratch 指向用户栈
    # 保存其他通用寄存器
    sd x1, 1*8(sp)
    # 跳过 sp(x2)，我们稍后会保存它
    sd x3, 3*8(sp)
    # 跳过 tp(x4)，应用程序不使用它
    # 保存 x5~x31
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # 我们可以自由使用 t0/t1/t2，因为它们已经保存在 TrapContext 中
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    # 从 sscratch 读取用户栈并将其保存在 TrapContext 中
    csrr t2, sscratch
    sd t2, 2*8(sp)
    # 将 kernel_satp 加载到 t0
    ld t0, 34*8(sp)
    # 将 trap_handler 加载到 t1
    ld t1, 36*8(sp)
    # 移动到 kernel_sp
    ld sp, 35*8(sp)
    # 切换到内核空间
    csrw satp, t0
    sfence.vma
    # 跳转到 trap_handler
    jr t1

__restore:
    # a0: 用户空间中的 *TrapContext（常量）; a1: 用户空间令牌
    # 切换到用户空间
    csrw satp, a1
    sfence.vma
    csrw sscratch, a0
    mv sp, a0
    # 现在 sp 指向用户空间中的 TrapContext，开始基于它恢复
    # 恢复 sstatus/sepc
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    # 恢复通用寄存器，除了 x0/sp/tp
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # 返回用户栈
    ld sp, 2*8(sp)
    sret
