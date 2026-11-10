        sys  READ_INT
        mov  r2, r0          ; Breite retten
        sys  READ_INT
        mul  r0, r2, r0
        sys  WRITE_INT
        halt
