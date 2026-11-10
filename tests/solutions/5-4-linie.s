; Variante: selten gebrauchte Werte in .data statt auf dem Stack
        sys READ_INT
        mov r4, r0               ; x0
        sys READ_INT
        mov r5, r0               ; y0
        sys READ_INT
        st r0, [ziel_x]
        sys READ_INT
        st r0, [ziel_y]
        sys READ_INT
        mov r7, r0               ; farbe
        ld r0, [ziel_x]
        sub r0, r4               ; x1 - x0
        mov r1, 1
        cmp r0, 0
        jge dx_ok
        neg r0
        mov r1, -1
dx_ok:
        st r0, [dx]
        st r1, [sx]
        ld r0, [ziel_y]
        sub r0, r5               ; y1 - y0
        mov r1, 1
        ld r2, [ziel_y]
        cmp r5, r2
        jlt sy_ok
        mov r1, -1
sy_ok:
        st r1, [sy]
        cmp r0, 0
        jlt dy_ok
        neg r0                   ; dy = -|y1 - y0|
dy_ok:
        st r0, [dy]
        ld r6, [dx]
        add r6, r0               ; fehler = dx + dy
loop:
        mov r0, r4
        mov r1, r5
        mov r2, r7
        call pixel
        ld r0, [ziel_x]
        cmp r4, r0
        jnz weiter
        ld r0, [ziel_y]
        cmp r5, r0
        jz ende
weiter:
        add r1, r6, r6           ; e2
        ld r2, [dy]
        cmp r1, r2
        jlt nur_y
        add r6, r2
        ld r3, [sx]
        add r4, r3
nur_y:
        ld r2, [dx]
        cmp r1, r2
        jgt loop
        add r6, r2
        ld r3, [sy]
        add r5, r3
        jmp loop
ende:
        halt
.data
ziel_x: .word 0
ziel_y: .word 0
dx:     .word 0
dy:     .word 0
sx:     .word 0
sy:     .word 0
