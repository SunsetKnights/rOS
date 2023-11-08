/*指定平台*/
OUTPUT_ARCH(riscv)
/*指定入口点位置，在qemuo中，从0x80000000开始，200000字节用于bootloader*/
ENTRY(_start)
BASE_ADDRESS = 0x80200000;
SECTIONS
{
    /* .表示当前位置，即从这里开始往下放置需要被链接的文件的段 */
    . = BASE_ADDRESS;
    /*skernel和ekernel是两个全局符号，表示kernel的开始和结束地址，下面的stext、etext，srodata、erodate同理*/
    /*不过目前这些全局符号没有用处*/
    skernel = .;

    /*下面放置各个段的位置*/
    stext = .;
    .text : {
        *(.text.entry)  /*.text.entry即entry.asm中第一行代码的位置*/
        *(.text .text.*)
    }
    /*ALIGN(4K)表示插入一些填充字节,直到当前位置在4K对齐*/
    . = ALIGN(4K);
    etext = .;

    srodata = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    . = ALIGN(4K);
    erodata = .;

    sdata = .;
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    . = ALIGN(4K);
    edata = .;

    .bss : {
        *(.bss.stack)   /*.bss.stack即entry.asm中，第一个栈的位置*/
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }
    . = ALIGN(4K);
    ebss = .;

    ekernel = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}