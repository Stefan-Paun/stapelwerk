        sys READ_INT
        mov r4, r0
schleife:
        cmp r4, 0
        jz fertig
        sys READ_INT
        mov r5, r0
        sys READ_INT
        mov r1, r0
        sys READ_INT
        mov r2, r0
        mov r0, r5
        call max3
        sys WRITE_INT
        sub r4, 1
        jmp schleife
fertig:
        halt
max3:
        push r4
        mov r4, r0
        cmp r1, r4
        jle .pruefe_c
        mov r4, r1
.pruefe_c:
        cmp r2, r4
        jle .fertig
        mov r4, r2               ; andere Reparatur: kein früher Ausstieg
.fertig:
        mov r0, r4
        pop r4
        ret
