; Golden test: every instruction and operand form, with independently computed words.
; Runner: crates/core/tests/asm_golden.rs (each expect-word comment checks the words of its line).
.equ N, 10
.equ BIG, 0x12345678
main:
    halt                              ; expect-word: 0x01000000
    nop                               ; expect-word: 0x02000000
    ret                               ; expect-word: 0x08000000
    sys WRITE_INT                     ; expect-word: 0x04000002
    sys read_int                      ; expect-word: 0x04000001
    SYS Rand                          ; expect-word: 0x04000008
    sys 99                            ; expect-word: 0x04000063
    sys 0xFFFF                        ; expect-word: 0x0400FFFF
    sys 0                             ; expect-word: 0x04000000
    mov r1, r2                        ; expect-word: 0x10120000
    mov sp, fp                        ; expect-word: 0x10980000
    mov r1, -32768                    ; expect-word: 0x11108000
    mov fp, 32767                     ; expect-word: 0x11807FFF
    MOV R3, 'A'                       ; expect-word: 0x11300041
    mov r4, '\n'                      ; expect-word: 0x1140000A
    mov r1, N * 2 + 1                 ; expect-word: 0x11100015
    mov r1, (1 + 2) * 3               ; expect-word: 0x11100009
    mov r1, 0b1010                    ; expect-word: 0x1110000A
    mov r1, 1_000                     ; expect-word: 0x111003E8
    mov r1, -7 / 2                    ; expect-word: 0x1110FFFD
    mov r1, -7 % 2                    ; expect-word: 0x1110FFFF
    mov r1, ~0                        ; expect-word: 0x1110FFFF
    mov r1, 1 << 4 | 1                ; expect-word: 0x11100011
    mov r1, 6 & 3 ^ 1                 ; expect-word: 0x11100003
    mov r1, 256 >> 2                  ; expect-word: 0x11100040
    mov r1, WHITE                     ; expect-word: 0x1110000F
    mov r1, SCREEN_W * SCREEN_H / 64  ; expect-word: 0x11100040
    movhi r1, 0xFFFF                  ; expect-word: 0x1210FFFF
    movhi r1, 0                       ; expect-word: 0x12100000
    li r2, BIG                        ; expect-word: 0x11205678 0x12201234
    li r2, -1                         ; expect-word: 0x1120FFFF 0x1220FFFF
    li r2, 0xFFFFFFFF                 ; expect-word: 0x1120FFFF 0x1220FFFF
    li r2, -2147483648                ; expect-word: 0x11200000 0x12208000
    li r0, main                       ; expect-word: 0x11000100 0x12000000
    li r0, later                      ; expect-word: 0x1100039C 0x12000000
    li r5, SCREEN + 64 * 2 + 3        ; expect-word: 0x1150F083 0x12500000
    lea r1, [fp - 8]                  ; expect-word: 0x1318FFF8
    lea r1, [SCREEN]                  ; expect-word: 0x131FF000
    ld r0, [fp - 8]                   ; expect-word: 0x5008FFF8
    ld r0, [0x4000]                   ; expect-word: 0x500F4000
    ld r0, [sp]                       ; expect-word: 0x50090000
    ld r0, [r1 + 32767]               ; expect-word: 0x50017FFF
    ld r0, [r1 - 32768]               ; expect-word: 0x50018000
    ld r0, [r1 + 2 * 3 - 1]           ; expect-word: 0x50010005
    ld r0, [r1 - 8 + 4]               ; expect-word: 0x5001FFFC
    ld r0, [0]                        ; expect-word: 0x500F0000
    ld r6, [INPUT + 4]                ; expect-word: 0x506F6004
    ldb r2, [r3 + 1]                  ; expect-word: 0x51230001
    ldbs r2, [r3 - 1]                 ; expect-word: 0x5223FFFF
    st r1, [fp + 8]                   ; expect-word: 0x54180008
    stb r1, [0xFFFF]                  ; expect-word: 0x551FFFFF
    st r7, [value]                    ; expect-word: 0x547F4000
    push fp                           ; expect-word: 0x58800000
    pop r7                            ; expect-word: 0x59700000
    add r1, r2, r3                    ; expect-word: 0x20123000
    add r1, r2                        ; expect-word: 0x20112000
    add r4, fp, -32768                ; expect-word: 0x30488000
    add r4, fp, 32767                 ; expect-word: 0x30487FFF
    add r5, 3                         ; expect-word: 0x30550003
    sub r1, r2, r3                    ; expect-word: 0x21123000
    sub r1, r2                        ; expect-word: 0x21112000
    sub r4, fp, -32768                ; expect-word: 0x31488000
    sub r4, fp, 32767                 ; expect-word: 0x31487FFF
    sub r5, 3                         ; expect-word: 0x31550003
    mul r1, r2, r3                    ; expect-word: 0x22123000
    mul r1, r2                        ; expect-word: 0x22112000
    mul r4, fp, -32768                ; expect-word: 0x32488000
    mul r4, fp, 32767                 ; expect-word: 0x32487FFF
    mul r5, 3                         ; expect-word: 0x32550003
    div r1, r2, r3                    ; expect-word: 0x23123000
    div r1, r2                        ; expect-word: 0x23112000
    div r4, fp, -32768                ; expect-word: 0x33488000
    div r4, fp, 32767                 ; expect-word: 0x33487FFF
    div r5, 3                         ; expect-word: 0x33550003
    divu r1, r2, r3                   ; expect-word: 0x24123000
    divu r1, r2                       ; expect-word: 0x24112000
    divu r4, fp, 0                    ; expect-word: 0x34480000
    divu r4, fp, 65535                ; expect-word: 0x3448FFFF
    divu r5, 3                        ; expect-word: 0x34550003
    rem r1, r2, r3                    ; expect-word: 0x25123000
    rem r1, r2                        ; expect-word: 0x25112000
    rem r4, fp, -32768                ; expect-word: 0x35488000
    rem r4, fp, 32767                 ; expect-word: 0x35487FFF
    rem r5, 3                         ; expect-word: 0x35550003
    remu r1, r2, r3                   ; expect-word: 0x26123000
    remu r1, r2                       ; expect-word: 0x26112000
    remu r4, fp, 0                    ; expect-word: 0x36480000
    remu r4, fp, 65535                ; expect-word: 0x3648FFFF
    remu r5, 3                        ; expect-word: 0x36550003
    and r1, r2, r3                    ; expect-word: 0x27123000
    and r1, r2                        ; expect-word: 0x27112000
    and r4, fp, 0                     ; expect-word: 0x37480000
    and r4, fp, 65535                 ; expect-word: 0x3748FFFF
    and r5, 3                         ; expect-word: 0x37550003
    or r1, r2, r3                     ; expect-word: 0x28123000
    or r1, r2                         ; expect-word: 0x28112000
    or r4, fp, 0                      ; expect-word: 0x38480000
    or r4, fp, 65535                  ; expect-word: 0x3848FFFF
    or r5, 3                          ; expect-word: 0x38550003
    xor r1, r2, r3                    ; expect-word: 0x29123000
    xor r1, r2                        ; expect-word: 0x29112000
    xor r4, fp, 0                     ; expect-word: 0x39480000
    xor r4, fp, 65535                 ; expect-word: 0x3948FFFF
    xor r5, 3                         ; expect-word: 0x39550003
    shl r1, r2, r3                    ; expect-word: 0x2A123000
    shl r1, r2                        ; expect-word: 0x2A112000
    shl r4, fp, 0                     ; expect-word: 0x3A480000
    shl r4, fp, 31                    ; expect-word: 0x3A48001F
    shl r5, 3                         ; expect-word: 0x3A550003
    shr r1, r2, r3                    ; expect-word: 0x2B123000
    shr r1, r2                        ; expect-word: 0x2B112000
    shr r4, fp, 0                     ; expect-word: 0x3B480000
    shr r4, fp, 31                    ; expect-word: 0x3B48001F
    shr r5, 3                         ; expect-word: 0x3B550003
    sar r1, r2, r3                    ; expect-word: 0x2C123000
    sar r1, r2                        ; expect-word: 0x2C112000
    sar r4, fp, 0                     ; expect-word: 0x3C480000
    sar r4, fp, 31                    ; expect-word: 0x3C48001F
    sar r5, 3                         ; expect-word: 0x3C550003
    cmp r1, r2                        ; expect-word: 0x2D012000
    cmp sp, -32768                    ; expect-word: 0x3D098000
    cmp sp, 32767                     ; expect-word: 0x3D097FFF
    tst r1, r2                        ; expect-word: 0x2E012000
    tst sp, 0                         ; expect-word: 0x3E090000
    tst sp, 65535                     ; expect-word: 0x3E09FFFF
    add r1, r2, -1                    ; expect-word: 0x3012FFFF
    add r1, r2, r3                    ; expect-word: 0x20123000
    sub sp, N                         ; expect-word: 0x3199000A
    and r1, 0xFF                      ; expect-word: 0x371100FF
    neg r1, r2                        ; expect-word: 0x40120000
    neg r1                            ; expect-word: 0x40110000
    not r3, r4                        ; expect-word: 0x41340000
    not r3                            ; expect-word: 0x41330000
func:
    jmp func                          ; expect-word: 0x60000000
    jz func                           ; expect-word: 0x601FFFFF
    jnz func                          ; expect-word: 0x602FFFFE
    jlt func                          ; expect-word: 0x603FFFFD
    jge func                          ; expect-word: 0x604FFFFC
    jle func                          ; expect-word: 0x605FFFFB
    jgt func                          ; expect-word: 0x606FFFFA
    jb func                           ; expect-word: 0x607FFFF9
    jae func                          ; expect-word: 0x608FFFF8
    jbe func                          ; expect-word: 0x609FFFF7
    ja func                           ; expect-word: 0x60AFFFF6
    js func                           ; expect-word: 0x60BFFFF5
    jns func                          ; expect-word: 0x60CFFFF4
    jo func                           ; expect-word: 0x60DFFFF3
    jno func                          ; expect-word: 0x60EFFFF2
    je func                           ; expect-word: 0x601FFFF1
    JNE func                          ; expect-word: 0x602FFFF0
    jz func + 4                       ; expect-word: 0x601FFFF0
    jmp later                         ; expect-word: 0x60000009
    jmp r1                            ; expect-word: 0x62100000
    call func                         ; expect-word: 0x610FFFEC
    call r2                           ; expect-word: 0x63200000
    jz 0x0100                         ; expect-word: 0x601FFF5E
three: nop ; expect-word: 0x02000000
    nop                               ; expect-word: 0x02000000
    nop                               ; expect-word: 0x02000000
    jz three                          ; expect-word: 0x601FFFFD
later:
.x:
    jmp .x                            ; expect-word: 0x60000000
    jmp later.x                       ; expect-word: 0x600FFFFF
    ld r1, [value.y]                  ; expect-word: 0x501F4004
    halt ; expect-word: 0x01000000
.data
value: .word 1
.y: .byte 2
