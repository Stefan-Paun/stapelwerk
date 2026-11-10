        mov r1, 0
lesen:
        sys READ_INT
        cmp r0, 0
        jz ausgeben
        push r0
        add r1, 1
        jmp lesen
ausgeben:
        cmp r1, 0
        jz fertig
        pop r0
        sys WRITE_INT
        sub r1, 1
        jmp ausgeben
fertig:
        halt
