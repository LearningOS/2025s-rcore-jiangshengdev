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
    # 此时 sp 指向用户空间的 TrapContext，sscratch 指向用户栈
    # 保存其他通用寄存器
    sd x1, 1*8(sp)
    # 跳过 sp(x2)，稍后保存
    sd x3, 3*8(sp)
    # 跳过 tp(x4)，应用不会用到
    # 保存 x5~x31
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # 可以自由使用 t0/t1/t2，因为它们已保存在 TrapContext 中
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    # 从 sscratch 读取用户栈并保存到 TrapContext
    csrr t2, sscratch
    sd t2, 2*8(sp)
    # 加载 kernel_satp 到 t0
    ld t0, 34*8(sp)
    # 加载 trap_handler 到 t1
    ld t1, 36*8(sp)
    # 切换到内核栈
    ld sp, 35*8(sp)
    # 切换到内核空间
    csrw satp, t0
    sfence.vma
    # 跳转到 trap_handler
    jr t1

__restore:
    # a0: 用户空间 TrapContext 指针（常量）；a1: 用户空间 token
    # 切换到用户空间
    csrw satp, a1
    sfence.vma
    csrw sscratch, a0
    mv sp, a0
    # 此时 sp 指向用户空间 TrapContext，开始基于其恢复
    # 恢复 sstatus/sepc
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    # 恢复除 x0/sp/tp 外的通用寄存器
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # 回到用户栈
    ld sp, 2*8(sp)
    sret
