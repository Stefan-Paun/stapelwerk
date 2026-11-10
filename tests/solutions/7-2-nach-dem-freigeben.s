; Nach dem Freigeben
; Liest Zahlen bis zur 0 in eine Liste, entfernt die negativen,
; gibt den Rest aus und gibt alles frei. Mit negativen Zahlen stürzt es ab.

        mov r4, 0                ; r4 = kopf (leere Liste)
lesen:
        sys READ_INT
        cmp r0, 0
        jz aussieben
        mov r5, r0               ; r5 = wert
        mov r0, 8
        sys ALLOC                ; r0 = neuer Knoten
        st r4, [r0]              ; next = kopf
        st r5, [r0 + 4]          ; wert
        mov r4, r0               ; kopf = neuer Knoten
        jmp lesen

aussieben:
        mov r5, 0                ; r5 = vorher (noch keiner)
        mov r6, r4               ; r6 = p = kopf
.pruefen:
        cmp r6, 0
        jz ausgeben
        ld r0, [r6 + 4]          ; p->wert
        cmp r0, 0
        jge .behalten
        ld r1, [r6]              ; r1 = p->next
        cmp r5, 0
        jz .neuer_kopf
        st r1, [r5]              ; vorher->next = p->next
        jmp .freigeben
.neuer_kopf:
        mov r4, r1               ; kopf = p->next
.freigeben:
        mov r0, r6
        ld r6, [r6]              ; erst weiter, dann freigeben
        sys FREE
        jmp .pruefen
.behalten:
        mov r5, r6               ; vorher = p
        ld r6, [r6]              ; p = p->next
        jmp .pruefen

ausgeben:
        cmp r4, 0
        jz fertig
        ld r0, [r4 + 4]
        sys WRITE_INT
        ld r5, [r4]              ; next merken, solange der Knoten noch gilt
        mov r0, r4
        sys FREE
        mov r4, r5
        jmp ausgeben
fertig:
        halt
