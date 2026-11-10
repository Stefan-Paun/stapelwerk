; andere Reparatur: r1 vom Aufrufer gesichert (push/pop um den Aufruf)
fib:
        push fp
        mov fp, sp
        sub sp, 4
        st r0, [fp - 4]
        cmp r0, 2
        jlt .ende
        sub r0, 1
        call fib
        mov r1, r0
        push r1
        ld r0, [fp - 4]
        sub r0, 2
        call fib
        pop r1
        add r0, r1
.ende:
        mov sp, fp
        pop fp
        ret
