# 实现特权级切换时，程序执行上下文的保存与恢复
.altmacro   # To use .rept cycle
.macro SAVE_REG n
    sd x\n, \n*8(sp)
.endm
.macro LOAD_REG n
    ld x\n, \n*8(sp)
.endm
    .section .text
    .global __savetrapsreg
    .global __restoretrapreg
    .align 2
__savetrapsreg:
    # switch sp and sscratch, before switch, sp is user stack point, sscratch is system stack point
    csrrw sp, sscratch, sp
    # system stack grows downward by 34 words, 0-31 save reg0-31, 32 save sstatus, 33 save sepc
    addi sp, sp, -34*8
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
    curr x1, sscratch
    sd x1, 2*8(sp)
    # set input argument of trap_handler(context: &mut TrapContext)
    # actually a0 is the x10, it's the first parameter in riscv function call, also, return value will put in this register too
    mv a0, sp
    call trap_handler

__restoretrapreg:
    mv sp, a0
    # restore sepc
    ld x1, 33*8(sp)
    csrw sepc, x1
    # restore sstatus
    ld x1, 32*8(sp)
    csrw sstatus, x1
    # restore sscratch
    ld x1, 2*8(sp)
    csrw sscratch, x1
    # restore x1,x3,x5-x31
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n,5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # restore system stack
    addi sp, sp, 34*8
    # switch user stack pointer and system stack pointer user
    csrrw sp, sscratch, sp
    sret