        sys  READ_INT
        mov  r2, 1           ; Maske
schleife:
        and  r3, r0, r2
        jz   null
        add  r1, 1
null:
        shl  r2, 1
        jnz  schleife
        mov  r0, r1
        sys  WRITE_INT
        halt
