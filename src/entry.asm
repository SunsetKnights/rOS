# 内核程序的入口点
.section .text.entry    # 段名称
.global _start          # 定义一个全局符号
_start:
    li x1,114514        # 将下北泽常数放入x1寄存器，li即立即数命令