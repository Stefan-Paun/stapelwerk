schleife:
        sys  READ_INT
        cmp  r0, 0
        jz   fertig
        sar  r0, r0, 1
        sys  WRITE_INT
        jmp  schleife
fertig:
        halt
