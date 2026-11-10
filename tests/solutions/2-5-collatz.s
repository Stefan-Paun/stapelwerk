        sys  READ_INT
        mov  r3, r0          ; n
schleife:
        mov  r0, r3
        sys  WRITE_INT
        cmp  r3, 1
        jz   fertig
        rem  r2, r3, 2
        cmp  r2, 0
        jz   gerade
        mul  r3, 3
        add  r3, 1
        jmp  weiter
gerade:
        div  r3, 2
weiter:
        add  r1, 1
        jmp  schleife
fertig:
        mov  r0, r1
        sys  WRITE_INT
        halt
