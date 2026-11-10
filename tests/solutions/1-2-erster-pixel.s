        sys  READ_INT
        mov  r1, r0          ; x
        sys  READ_INT
        mov  r2, r0          ; y
        sys  READ_INT
        mov  r3, r0          ; farbe
        mul  r2, 64
        add  r2, r1
        li   r4, SCREEN
        add  r2, r4
        stb  r3, [r2]
        halt
