        sys  READ_INT
schleife:
        cmp  r0, 0
        jle  fertig
        sys  WRITE_INT
        sub  r0, 2
        jmp  schleife
fertig:
        halt
