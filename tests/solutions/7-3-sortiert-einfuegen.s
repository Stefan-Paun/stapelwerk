; Variante: Zeiger auf Zeiger (Hinweis 3), kopf in .data
lesen:
        sys READ_INT
        cmp r0, 0
        jz ausgeben
        mov r5, r0               ; x
        mov r0, 8
        sys ALLOC
        st r5, [r0 + 4]
        mov r1, kopf
suchen:
        ld r2, [r1]
        cmp r2, 0
        jz einhaengen
        ld r3, [r2 + 4]
        cmp r3, r5
        jgt einhaengen
        mov r1, r2
        jmp suchen
einhaengen:
        st r2, [r0]
        st r0, [r1]
        jmp lesen
ausgeben:
        ld r4, [kopf]
weiter:
        cmp r4, 0
        jz fertig
        ld r0, [r4 + 4]
        sys WRITE_INT
        ld r5, [r4]
        mov r0, r4
        sys FREE
        mov r4, r5
        jmp weiter
fertig:
        halt
.data
kopf:   .word 0
