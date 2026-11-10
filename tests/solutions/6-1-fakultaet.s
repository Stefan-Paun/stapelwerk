; Variante: n in r4 (callee-saved) statt lokaler Variable
fak:
        push r4
        mov r4, r0
        cmp r0, 1
        jle basis
        sub r0, 1
        call fak
        mul r0, r4
        pop r4
        ret
basis:
        mov r0, 1
        pop r4
        ret
