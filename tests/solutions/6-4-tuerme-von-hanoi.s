; Variante: Argumente als lokale Variablen im Rahmen (fp)
        sys READ_INT
        push r0                  ; n kurz parken
        sys READ_INT
        mov r1, r0
        sys READ_INT
        mov r2, r0
        mov r3, 6
        sub r3, r1
        sub r3, r2
        pop r0
        call hanoi
        halt
hanoi:
        cmp r0, 0
        jz .zurueck
        push fp
        mov fp, sp
        sub sp, 16
        st r0, [fp - 4]
        st r1, [fp - 8]
        st r2, [fp - 12]
        st r3, [fp - 16]
        sub r0, 1
        ld r2, [fp - 16]
        ld r3, [fp - 12]
        call hanoi
        ld r0, [fp - 8]
        sys WRITE_INT
        ld r0, [fp - 12]
        sys WRITE_INT
        ld r0, [fp - 4]
        sub r0, 1
        ld r1, [fp - 16]
        ld r2, [fp - 12]
        ld r3, [fp - 8]
        call hanoi
        mov sp, fp
        pop fp
.zurueck:
        ret
