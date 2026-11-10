        sys  READ_INT
        mov  r1, r0          ; a
        sys  READ_INT
        mov  r2, r0          ; b
        mov  r0, r1
        add  r0, r2          ; Kurzform
        sys  WRITE_INT
        mov  r0, r1
        sub  r0, r2
        sys  WRITE_INT
        halt
