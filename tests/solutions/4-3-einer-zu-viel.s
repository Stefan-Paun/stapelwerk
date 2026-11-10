; andere Reparatur: jle bleibt, Grenze ist anzahl - 1
        ld r1, [ANZAHL]
        sub r1, 1
        mov r2, 0
        mov r3, 0
schleife:
        mul r4, r2, 4
        ld r0, [r4 + WERTE]
        add r3, r0
        add r2, 1
        cmp r2, r1
        jle schleife
        st r3, [SUMME]
        halt
