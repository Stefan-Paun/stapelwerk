; Golden test: one error per line (assembled without library and filter).
; Kommentar mit Umlauten: Zähler, Größe, 😀 – davor
.x: nop                             ; expect-error: asm.local_label_without_global 3:1
main:
    ad r1, r2                       ; expect-error: asm.unknown_mnemonic 5:5 name=ad suggestion=add
    jeq main                        ; expect-error: asm.unknown_mnemonic 6:5 name=jeq suggestion=je
    frobnicate                      ; expect-error: asm.unknown_mnemonic 7:5 name=frobnicate
    .wrod 1                         ; expect-error: asm.unknown_mnemonic 8:5 name=.wrod suggestion=.word
    mov r8, 1                       ; expect-error: asm.unknown_register 9:9 name=r8
    add r1, r2, r12                 ; expect-error: asm.unknown_register 10:17 name=r12
    push pc                         ; expect-error: asm.unknown_register 11:10 name=pc
    add r1                          ; expect-error: asm.operand_count 12:5 expected=2-3 got=1
    halt r1                         ; expect-error: asm.operand_count 13:5 expected=0 got=1
    cmp r1, r2, r3                  ; expect-error: asm.operand_count 14:5 expected=2 got=3
    ld r1, r2                       ; expect-error: asm.operand_type 15:12 expected=memory
    jz r1                           ; expect-error: asm.operand_type 16:8 expected=label
    movhi r1, r2                    ; expect-error: asm.operand_type 17:15 expected=immediate
    add r1, r2,                     ; expect-error: asm.operand_type 18:16 expected=register
    add r1, r2 + 1                  ; expect-error: asm.operand_type 19:13 expected=immediate
    mov r1, 70000                   ; expect-error: asm.imm_range 20:13 value=70000 min=-32768 max=32767 hint=li
    and r1, -1                      ; expect-error: asm.imm_range 21:13 value=-1 min=0 max=65535
    shl r1, 32                      ; expect-error: asm.imm_range 22:13 value=32 min=0 max=31
    ld r1, [fp + 40000]             ; expect-error: asm.imm_range 23:18 value=40000
    ld r1, [0x10000]                ; expect-error: asm.imm_range 24:13 max=65535
    sys 70000                       ; expect-error: asm.imm_range 25:9
    li r1, 0x100000000              ; expect-error: asm.imm_range 26:12 max=4294967295
    jmp 0x300000                    ; expect-error: asm.imm_range 27:9
    jmp nowhere                     ; expect-error: asm.undefined_symbol 28:9 name=nowhere
    sys PRINT                       ; expect-error: asm.undefined_symbol 29:9 name=PRINT
    jmp .nolocal                    ; expect-error: asm.undefined_symbol 30:9 name=.nolocal
main: nop                           ; expect-error: asm.duplicate_label 31:1 name=main first_line=4
add: nop                            ; expect-error: asm.reserved_name 32:1 name=add
__mine: nop                         ; expect-error: asm.reserved_name 33:1 name=__mine
.equ SCREEN, 5                      ; expect-error: asm.reserved_name 34:6 name=SCREEN
    mov r1, 0x                      ; expect-error: asm.bad_number 35:13
    mov r1, 1__0                    ; expect-error: asm.bad_number 36:13
    mov r1, 'ab'                    ; expect-error: asm.bad_string 37:13
    mov r1, 5 @ 3                   ; expect-error: asm.unexpected_char 38:15 ch=@
    mov r1, 5 6                     ; expect-error: asm.unexpected_char 39:15 ch=6
    mov r1, LATER                   ; expect-error: asm.needs_constant 40:13
.equ LATER, 7
.space 2                            ; expect-error: asm.data_in_text 42:1
    mov r1, 1 / (2 - 2)             ; expect-error: asm.expr_div_zero 43:13
    mov r1, 0x7FFFFFFFFFFFFFFF + 1  ; expect-error: asm.expr_overflow 44:13
    .string "x"                     ; expect-error: asm.data_in_text 45:5
    jmp .end
.end: halt
.data
    nop                             ; expect-error: asm.code_in_data 49:5
buf: .space later_label - buf       ; expect-error: asm.needs_constant 50:13
    .byte 'ä', '😀', 300             ; expect-error: asm.imm_range 51:16 value=128512
    .word 'ä', '😀', 1 << 40         ; expect-error: asm.imm_range 52:22 value=1099511627776
    .align 3                        ; expect-error: asm.imm_range 53:12 min=1 max=256
    .string 5                       ; expect-error: asm.operand_type 54:13 expected=string
    .word                           ; expect-error: asm.operand_count 55:5 expected=1+ got=0
later_label: .byte 1
