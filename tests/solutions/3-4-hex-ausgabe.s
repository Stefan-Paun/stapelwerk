        sys  READ_INT
        mov  r1, r0
        mov  r2, 8
schleife:
        shr  r0, r1, 28
        cmp  r0, 9
        jgt  buchstabe
        add  r0, '0'
        jmp  aus
buchstabe:
        add  r0, 'A' - 10
aus:
        sys  WRITE_CHAR
        shl  r1, 4
        sub  r2, 1
        jnz  schleife
        halt
