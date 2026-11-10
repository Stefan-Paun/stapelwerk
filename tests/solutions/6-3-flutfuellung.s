; Variante: alle vier Werte als Argumente (x, y, alt, neu), r4-r7 gesichert
        sys READ_INT
        mov r4, r0
        sys READ_INT
        mov r5, r0
        sys READ_INT
        mov r3, r0               ; neu
        mul r1, r5, 64
        add r1, r4
        li r2, SCREEN
        add r1, r2
        ldb r2, [r1]             ; alt
        cmp r2, r3
        jz fertig
        mov r0, r4
        mov r1, r5
        call fuelle
fertig:
        halt
fuelle:
        cmp r0, 64
        jae .raus
        cmp r1, 64
        jae .raus
        push r4
        push r5
        push r6
        push r7
        mov r4, r0
        mov r5, r1
        mov r6, r2
        mov r7, r3
        mul r1, r5, 64
        add r1, r4
        li r0, SCREEN
        add r1, r0
        ldb r0, [r1]
        cmp r0, r6
        jnz .ende
        stb r7, [r1]
        add r0, r4, 1
        mov r1, r5
        mov r2, r6
        mov r3, r7
        call fuelle
        sub r0, r4, 1
        mov r1, r5
        mov r2, r6
        mov r3, r7
        call fuelle
        mov r0, r4
        add r1, r5, 1
        mov r2, r6
        mov r3, r7
        call fuelle
        mov r0, r4
        sub r1, r5, 1
        mov r2, r6
        mov r3, r7
        call fuelle
.ende:
        pop r7
        pop r6
        pop r5
        pop r4
.raus:
        ret
