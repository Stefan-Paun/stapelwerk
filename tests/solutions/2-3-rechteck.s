        sys  READ_INT
        mov  r1, r0          ; x
        li   r7, SCREEN
        add  r1, r7          ; x + SCREEN
        sys  READ_INT
        mov  r2, r0          ; y
        sys  READ_INT
        mov  r3, r0          ; breite
        sys  READ_INT
        mov  r4, r0          ; hoehe
        sys  READ_INT        ; farbe
        mov  r5, 0           ; j
zeile:
        cmp  r5, r4
        jge  fertig
        mov  r6, 0           ; i
spalte:
        cmp  r6, r3
        jge  naechste
        add  r7, r2, r5
        mul  r7, 64
        add  r7, r1
        add  r7, r6
        stb  r0, [r7]
        add  r6, 1
        jmp  spalte
naechste:
        add  r5, 1
        jmp  zeile
fertig:
        halt
