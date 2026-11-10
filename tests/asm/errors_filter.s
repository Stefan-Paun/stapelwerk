; Golden test: allow/forbid filter.
; golden-filter: forbid mul jnz li
main: mul r1, r2                    ; expect-error: asm.forbidden_instruction 3:7 mnemonic=mul
    jne main                        ; expect-error: asm.forbidden_instruction 4:5 mnemonic=jnz
    li r1, 5                        ; expect-error: asm.forbidden_instruction 5:5 mnemonic=li
    add r1, 1
    halt
