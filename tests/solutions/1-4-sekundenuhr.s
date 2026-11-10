        sys  READ_INT
        mov  r1, r0
        div  r2, r1, 60      ; Minuten gesamt
        rem  r3, r1, 60      ; Sekunden
        div  r0, r2, 60      ; Stunden
        sys  WRITE_INT
        rem  r0, r2, 60      ; Minuten
        sys  WRITE_INT
        mov  r0, r3
        sys  WRITE_INT
        halt
