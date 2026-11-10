; Golden test: data segment overflow.
main: halt
.data
    .space 8000
    .space 200                      ; expect-error: asm.section_overflow 5:5 section=data size=8201 max=8192
    .byte 1
