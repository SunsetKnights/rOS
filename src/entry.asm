    # 内核程序的入口点
    .section .text.entry    # 定义一个段名称， 从这里往下到下一个.section都属于本段（？），在ld文件中可以指定该段的位置，本项目中，这个段在text段的最开始位置
    .global _start          # 定义一个全局符号
_start:
    # 设置初始的栈
    la sp, boot_stack_top   # 将内核中分配的第一个栈的栈顶地址放到sp寄存器，la即load
    call rust_main

    .section .bss.stack     # 初始的stack段，由于栈事实上是存放在内存中，所以存放在bss段
    .global boot_stack_lower_limit
boot_stack_lower_limit:
    .space 4096*16          # 占用4KB*16=64KB的内存作为初始栈的极限大小
    .global boot_stack_top  # 程序实际执行中，栈是从高地址向低地址增长的，但是内存分配的时候，是从低地址向高地址分配的，所以栈能达到的极限大小在前，栈顶在后
boot_stack_top:
