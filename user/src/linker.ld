/*指定平台*/
OUTPUT_ARCH(riscv)
/*指定入口点位置*/
ENTRY(_start)
BASE_ADDRESS = 0x80400000;
SECTIONS
{
    /* .表示当前位置，即从这里开始往下放置需要被链接的文件的段 */
    . = BASE_ADDRESS;

    /*下面放置各个段的位置*/
    .text : {
        *(.text.entry)  /*.text.entry即entry.asm中第一行代码的位置*/
        *(.text .text.*)
    }
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    .bss : {
        u_sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        u_ebss = .;
    }
    /DISCARD/ : {   /*这些段将不会链接到输出文件*/
        *(.eh_frame)
        *(.debug*)
    }
}