; Variante: Ausgeben und Freigeben in einer Schleife
        mov r4, 0
lesen:
        sys READ_INT
        cmp r0, 0
        jz ausgeben
        mov r5, r0
        mov r0, 8
        sys ALLOC
        st r4, [r0]
        st r5, [r0 + 4]
        mov r4, r0
        jmp lesen
ausgeben:
        cmp r4, 0
        jz fertig
        ld r0, [r4 + 4]
        sys WRITE_INT
        ld r5, [r4]
        mov r0, r4
        sys FREE
        mov r4, r5
        jmp ausgeben
fertig:
        halt
