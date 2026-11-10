        sys  READ_INT
        mov  r1, r0
        sys  READ_INT
        mul  r2, r0, 64
        add  r2, r1
        li   r3, SCREEN
        add  r2, r3
        sys  READ_INT
        mov  r1, r0          ; farbe
        mov  r4, 8
zeile:
        sys  READ_INT
        mov  r6, 8
        mov  r5, r2
bit:
        tst  r0, 0x80
        jz   leer
        stb  r1, [r5]
leer:
        add  r5, 1
        shl  r0, 1
        sub  r6, 1
        jnz  bit
        add  r2, 64
        sub  r4, 1
        jnz  zeile
        halt
