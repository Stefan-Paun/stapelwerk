; Zeiger statt Index
        ld r1, [ANZAHL]
        mov r2, WERTE
        ld r3, [r2]
schleife:
        sub r1, 1
        jz fertig
        add r2, 4
        ld r0, [r2]
        cmp r0, r3
        jle schleife
        mov r3, r0
        jmp schleife
fertig:
        st r3, [MAX]
        halt
