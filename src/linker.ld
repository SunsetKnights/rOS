/*指定平台*/
OUTPUT_ARCH(riscv)
/*指定入口点*/
ENTRY(_start)
BASE_ADDRESS = 0x80200000;
SECTIONS
{
    /* .表示当前位置，即从这里开始往下放置需要被链接的文件的段 */
    . = BASE_ADDRESS;
    skernel = .;

}