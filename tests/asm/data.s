; Golden test: data directives, labels and alignment.
; "; expect-bytes: ADDR hh hh ..." checks the data segment, "; expect-symbol: NAME VALUE" the symbol table.
.equ COUNT, 3
main:   halt                         ; expect-word: 0x01000000
        .align 8                     ; padding with nop in .text
next:   nop                          ; expect-symbol: next 0x0108
.data
bytes:  .byte 1, -1, 255, 'A', COUNT ; expect-bytes: 0x4000 01 FF FF 41 03
words:  .word 0x11223344, -1         ; expect-bytes: 0x4008 44 33 22 11 FF FF FF FF
str:    .string "Hi\n"               ; expect-bytes: 0x4010 48 69 0A 00
asc:    .ascii "\x41\"\\"             ; expect-bytes: 0x4014 41 22 5C
        .space 2                     ; expect-bytes: 0x4017 00 00
al:     .align 4                     ; expect-symbol: al 0x401C
        .byte 9                      ; expect-bytes: 0x401C 09
lw:                                  ; label before an aligned .word
        .word lw, next, end          ; expect-bytes: 0x4020 20 40 00 00 08 01 00 00 10 01 00 00
umlaut: .string "Grüße"              ; expect-bytes: 0x402C 47 72 C3 BC C3 9F 65 00
.text
loc2:   nop                          ; expect-symbol: loc2 0x010C
end:                                 ; expect-symbol: end 0x0110
