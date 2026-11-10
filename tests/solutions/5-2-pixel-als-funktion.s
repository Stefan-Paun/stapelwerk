; mit Vorzeichen geprüft, aber vollständig
pixel:
        cmp r0, 0
        jlt .ende
        cmp r0, 64
        jge .ende
        cmp r1, 0
        jlt .ende
        cmp r1, 64
        jge .ende
        shl r3, r1, 6
        add r3, r0
        li r1, SCREEN
        add r3, r1
        stb r2, [r3]
.ende:
        ret
