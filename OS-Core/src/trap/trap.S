# 实现特权级切换时，程序执行上下文的保存与恢复
.altmacro   # To use .rept cycle
.macro SAVE_REG n
    sd x\n, \n*8(sp)
.endm
.macro LOAD_REG n
    ld x\n, \n*8(sp)
.endm
    .section .text.trampoline
    .global __savetrapsreg
    .global __restoretrapreg
    .align 2
__savetrapsreg:
    # switch sp and sscratch, before switch, sp is user stack point, sscratch is TrapContext ptr in user space
    csrrw sp, sscratch, sp
    # reg0 is always 0, not save
    # reg2 is sp, not save now
    # reg4 is Thread Pointer, not save 
    SAVE_REG 1
    SAVE_REG 3
    .set n, 5
    .rept 27
        SAVE_REG %n
        .set n, n+1
    .endr
    # save sstatus
    csrr x1, sstatus
    sd x1, 32*8(sp)
    # save sepc
    csrr x1, sepc
    sd x1, 33*8(sp)
    # save sscratch (user stack pointer)
    csrr x1, sscratch
    sd x1, 2*8(sp)
    # load kernel_satp
    ld t0, 34*8(sp)
    # load trap_handler
    ld t1, 36*8(sp)
    # load kernel stack
    ld sp, 35*8(sp)
    # set kernel satp (entry kernel space)
    # ************** call update_user_task_run_time ********************
    csrw satp, t0
    # clear TLB
    sfence.vma
    jr t1

__restoretrapreg:
    # ************* call save_leave_kernel_time **********************
    # a0: user space TrapContext
    # a1: user space satp
    csrw satp, a1
    # clear TLB
    sfence.vma
    # put TrapContext ptr in sscratch
    csrw sscratch, a0
    # use TrapContext
    mv sp, a0
    # restore sepc
    ld x1, 33*8(sp)
    csrw sepc, x1
    # restore sstatus
    ld x1, 32*8(sp)
    csrw sstatus, x1
    # restore x1,x3,x5-x31
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n,5
    .rept 27
        LOAD_REG %n
        .set n, n+1
    .endr
    # switch to user stack
    ld sp, 2*8(sp)
    sret

__save_kernel_trap_reg:
    addi sp, sp, -34*8
    sd x1, 1*8(sp)
    sd x3, 3*8(sp)
    .set n, 5
    .rept 27
        SAVE_REG %n
        .set n, n+1
    .endr
    csrr t0, sstatus
    sd t0, 32*8(sp)
    csrr t0, sepc
    sd t0, 33*8(sp)
    csrr t0, sscratch
    jalr ra, 0(t0)
__restore_kernel_trap_reg:
    ld t0, 32*8(sp)
    csrw sstatus, t0
    ld t0, 33*8(sp)
    csrw sepc, t0
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_REG %n
        .set n, n+1
    .endr
    addi sp, sp, 34*8
    sret
