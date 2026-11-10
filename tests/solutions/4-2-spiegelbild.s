; x/y-Indizes statt zwei Zeigern
        mov r6, 0                ; y
zeile:
        mov r5, 0                ; x
paar:
        li r2, SCREEN
        shl r1, r6, 6
        add r1, r5
        add r1, r2               ; links
        shl r3, r6, 6
        add r3, 63
        sub r3, r5
        add r3, r2               ; rechts
        ldb r0, [r1]
        ldb r4, [r3]
        stb r4, [r1]
        stb r0, [r3]
        add r5, 1
        cmp r5, 32
        jlt paar
        add r6, 1
        cmp r6, 64
        jlt zeile
        halt
