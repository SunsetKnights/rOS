.altmacro   # To use .rept cycle
.macro SAVE_S_REG n
    sd s\n, (\n+2)*8(a0)
.endm
.macro LOAD_S_REG n
    ld s\n, (\n+2)*8(a1)
.endm
    .section .text
    .global __switch
__switch:
    # __switch(
    #     current_task_context_ptr: *mut TaskContext,
    #     next_task_context_ptr: *const TaskContext,
    # )
    sd sp, 8(a0)
    sd ra, 0(a0)
    .set n, 0
    .rept 12
        SAVE_S_REG %n
        .set n, n+1
    .endr

    ld ra, 0(a1)
    .set n, 0
    .rept 12
        LOAD_S_REG %n
        .set n, n+1
    .endr
    ld sp, 8(a1)
    ret