        sys  READ_INT
        mov  r1, r0
        sys  READ_INT
        mov  r2, r0
        cmp  r1, r2
        jge  erste
        mov  r0, r2
        jmp  aus
erste:
        mov  r0, r1
aus:
        sys  WRITE_INT
        halt
