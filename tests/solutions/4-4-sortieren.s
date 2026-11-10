        ld r1, [ANZAHL]
        mov r2, 1
aussen:
        cmp r2, r1
        jge fertig
        mul r3, r2, 4
        ld r4, [r3 + WERTE]
        sub r5, r3, 4
innen:
        cmp r5, 0
        jlt einsetzen
        ld r6, [r5 + WERTE]
        cmp r6, r4
        jle einsetzen
        st r6, [r5 + WERTE + 4]
        sub r5, 4
        jmp innen
einsetzen:
        st r4, [r5 + WERTE + 4]
        add r2, 1
        jmp aussen
fertig:
        halt
