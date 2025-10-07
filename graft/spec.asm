; M = Mode
; O = Opcode
; I = 0/1 Is argument B an immediate value bit
; D = Destination register
; A = Argument A
; B = Argument B
; Z = Immediate value

; Instruction encoding if not an immediate value:
; MMIDDDDD OOOOOOOO AAAAAAAA BBBBBBBB
; Instruction encoding if an immediate value:
; MMIDDDDD OOOOOOOO AAAAAAAA BBBBBBBB ZZZZZZZZ ZZZZZZZZ ZZZZZZZZ ZZZZZZZZ

#once

#bankdef main
{
    #bits 32
    #addr 0x00000000
    #size 0xffffffff
    #outp 0
}

#bank main

#subruledef register
{
    ; [r] - caller saved
    ; [e] - callee saved
    zr  => 0x00 ; zero register
    ra  => 0x01 ; [r] return address
    sp  => 0x02 ; stack pointer
    gp  => 0x03 ; global pointer
    tp  => 0x04 ; thread pointer
    t0  => 0x05 ; [r] temporary 0
    t1  => 0x06 ; [r] temporary 1
    t2  => 0x07 ; [r] temporary 2
    t3  => 0x08 ; [r] temporary 3
    t4  => 0x09 ; [r] temporary 4
    t5  => 0x0a ; [r] temporary 5
    t6  => 0x0b ; [r] temporary 6
    fp  => 0x0c ; [e] frame pointer (same as s0)
    s0  => 0x0c ; [e] saved 0 (same as fp)
    s1  => 0x0d ; [e] saved 1
    s2  => 0x0e ; [e] saved 2
    s3  => 0x0f ; [e] saved 3
    s4  => 0x10 ; [e] saved 4
    s5  => 0x11 ; [e] saved 5
    s6  => 0x12 ; [e] saved 6
    s7  => 0x13 ; [e] saved 7
    s8  => 0x14 ; [e] saved 8
    s9  => 0x15 ; [e] saved 9
    s10 => 0x16 ; [e] saved 10
    s11 => 0x17 ; [e] saved 11
    a0  => 0x18 ; [r] function argument 0 / return value 0
    a1  => 0x19 ; [r] function argument 1 / return value 1
    a2  => 0x1a ; [r] function argument 2
    a3  => 0x1b ; [r] function argument 3
    a4  => 0x1c ; [r] function argument 4
    a5  => 0x1d ; [r] function argument 5
    a6  => 0x1e ; [r] function argument 6
    a7  => 0x1f ; [r] function argument 7

    r0  => 0x00
    r1  => 0x01
    r2  => 0x02
    r3  => 0x03
    r4  => 0x04
    r5  => 0x05
    r6  => 0x06
    r7  => 0x07
    r8  => 0x08
    r9  => 0x09
    r10 => 0x0a
    r11 => 0x0b
    r12 => 0x0c
    r13 => 0x0d
    r14 => 0x0e
    r15 => 0x0f
    r16 => 0x10
    r17 => 0x11
    r18 => 0x12
    r19 => 0x13
    r20 => 0x14
    r21 => 0x15
    r22 => 0x16
    r23 => 0x17
    r24 => 0x18
    r25 => 0x19
    r26 => 0x1a
    r27 => 0x1b
    r28 => 0x1c
    r29 => 0x1d
    r30 => 0x1e
    r31 => 0x1f
}

#subruledef immediate
{
    {immediate: i32}  => immediate
}

#ruledef
{
    nop => 0x00000000

    halt {a: register} => (0`2 @ 0b0 @ 0`5) @ 0x01 @ a @ 0x00

    ; console (utf-8)
    pr {a: register}, {b: register}  => (0`2 @ 0b0 @ 0`5) @ 0x02 @ a @ b
    epr {a: register}, {b: register}  => (0`2 @ 0b0 @ 0`5) @ 0x03 @ a @ b

    ; time
    time {d1: register}, {d2: register}, {d3: register}, {d4: register} =>
        (0`2 @ 0b1 @ 0`5) @ 0x04 @ d1 @ d2 @ d3 @ d4 @ 0x00 @ 0x00

    ; read program counter to register
    rdpc {d: register} => (0`2 @ 0b0 @ d`5) @ 0x05 @ 0x00 @ 0x00

    kbrd {d: register} => (0`2 @ 0b0 @ d`5) @ 0x06 @ 0x00 @ 0x00

    setgfx {a: register} => (0`2 @ 0b0 @ 0`5) @ 0x07 @ a @ 0x00
    setgfx {i: immediate} => (0`2 @ 0b1 @ 0`5) @ 0x07 @ 0x00 @ 0x00 @ i
    draw => (0`2 @ 0b0 @ 0`5) @ 0x08 @ 0x00 @ 0x00

    ; how much time to sleep in ms
    sleep {a: register}, {b: register} => (0`2 @ 0b0 @ 0`5) @ 0x09 @ a @ b
    sleep {i: immediate} => (0`2 @ 0b1 @ 0`5) @ 0x09 @ 0x00 @ 0x00 @ i

    ; ld mem

    ld {d: register}, mem[{a: register}] =>
        (0`2 @ 0b0 @ d`5) @ 0x20 @ a @ 0x00
    ld {d: register}, mem[{i: immediate}] =>
        (0`2 @ 0b1 @ d`5) @ 0x20 @ 0x00 @ 0x00 @ i

    ld.w {d: register}, mem[{a: register}] =>
        (0`2 @ 0b0 @ d`5) @ 0x21 @ a @ 0x00
    ld.w {d: register}, mem[{i: immediate}] =>
        (0`2 @ 0b1 @ d`5) @ 0x21 @ 0x00 @ 0x00 @ i

    ld.b {d: register}, mem[{a: register}] =>
        (0`2 @ 0b0 @ d`5) @ 0x22 @ a @ 0x00
    ld.b {d: register}, mem[{i: immediate}] =>
        (0`2 @ 0b1 @ d`5) @ 0x22 @ 0x00 @ 0x00 @ i

    ; ld storage

    ld {d: register}, st[{a: register}] =>
        (0`2 @ 0b0 @ d`5) @ 0x23 @ a @ 0x00
    ld {d: register}, st[{i: immediate}] =>
        (0`2 @ 0b1 @ d`5) @ 0x23 @ 0x00 @ 0x00 @ i

    ld.w {d: register}, st[{a: register}] =>
        (0`2 @ 0b0 @ d`5) @ 0x24 @ a @ 0x00
    ld.w {d: register}, st[{i: immediate}] =>
        (0`2 @ 0b1 @ d`5) @ 0x24 @ 0x00 @ 0x00 @ i

    ld.b {d: register}, st[{a: register}] =>
        (0`2 @ 0b0 @ d`5) @ 0x25 @ a @ 0x00
    ld.b {d: register}, st[{i: immediate}] =>
        (0`2 @ 0b1 @ d`5) @ 0x25 @ 0x00 @ 0x00 @ i

    ; st mem

    str mem[{d: register}], {a: register} =>
        (0`2 @ 0b0 @ d`5) @ 0x26 @ a @ 0x00
    str mem[{d: register}], {i: immediate} =>
        (0`2 @ 0b1 @ d`5) @ 0x26 @ 0x00 @ 0x00 @ i

    str.w mem[{d: register}], {a: register} =>
        (0`2 @ 0b0 @ d`5) @ 0x27 @ a @ 0x00
    str.w mem[{d: register}], {i: immediate} =>
        (0`2 @ 0b1 @ d`5) @ 0x27 @ 0x00 @ 0x00 @ i

    str.b mem[{d: register}], {a: register} =>
        (0`2 @ 0b0 @ d`5) @ 0x28 @ a @ 0x00
    str.b mem[{d: register}], {i: immediate} =>
        (0`2 @ 0b1 @ d`5) @ 0x28 @ 0x00 @ 0x00 @ i

    ; st storage

    str st[{d: register}], {a: register} =>
        (0`2 @ 0b0 @ d`5) @ 0x29 @ a @ 0x00
    str st[{d: register}], {i: immediate} =>
        (0`2 @ 0b1 @ d`5) @ 0x29 @ 0x00 @ 0x00 @ i

    str.w st[{d: register}], {a: register} =>
        (0`2 @ 0b0 @ d`5) @ 0x2a @ a @ 0x00
    str.w st[{d: register}], {i: immediate} =>
        (0`2 @ 0b1 @ d`5) @ 0x2a @ 0x00 @ 0x00 @ i

    str.b st[{d: register}], {a: register} =>
        (0`2 @ 0b0 @ d`5) @ 0x2b @ a @ 0x00
    str.b st[{d: register}], {i: immediate} =>
        (0`2 @ 0b1 @ d`5) @ 0x2b @ 0x00 @ 0x00 @ i

    ; todo draw

    ;
    ; math
    ;

    nand {d: register}, {a: register}, {b: register} =>
        (1`2 @ 0b0 @ d`5) @ 0x00 @ a @ b

    or {d: register}, {a: register}, {b: register} =>
        (1`2 @ 0b0 @ d`5) @ 0x01 @ a @ b

    and {d: register}, {a: register}, {b: register} =>
        (1`2 @ 0b0 @ d`5) @ 0x02 @ a @ b

    nor {d: register}, {a: register}, {b: register} =>
        (1`2 @ 0b0 @ d`5) @ 0x03 @ a @ b

    add {d: register}, {a: register}, {b: register} =>
        (1`2 @ 0b0 @ d`5) @ 0x04 @ a @ b

    sub {d: register}, {a: register}, {b: register} =>
        (1`2 @ 0b0 @ d`5) @ 0x05 @ a @ b

    xor {d: register}, {a: register}, {b: register} =>
        (1`2 @ 0b0 @ d`5) @ 0x06 @ a @ b

    lsl {d: register}, {a: register}, {b: register} =>
        (1`2 @ 0b0 @ d`5) @ 0x07 @ a @ b

    lsr {d: register}, {a: register}, {b: register} =>
        (1`2 @ 0b0 @ d`5) @ 0x08 @ a @ b

    mul {d: register}, {a: register}, {b: register} =>
        (1`2 @ 0b0 @ d`5) @ 0x09 @ a @ b

    cmp {d: register}, {a: register}, {b: register} =>
        (1`2 @ 0b0 @ d`5) @ 0x0a @ a @ b

    div {d: register}, {a: register}, {b: register} =>
        (1`2 @ 0b0 @ d`5) @ 0x0b @ a @ b

    rem {d: register}, {a: register}, {b: register} =>
        (1`2 @ 0b0 @ d`5) @ 0x0c @ a @ b

    ;
    ; math immediate
    ;

    nand {d: register}, {a: register}, {i: immediate} =>
        (1`2 @ 0b1 @ d`5) @ 0x00 @ a @ 0x00 @ i

    or {d: register}, {a: register}, {i: immediate} =>
        (1`2 @ 0b1 @ d`5) @ 0x01 @ a @ 0x00 @ i

    and {d: register}, {a: register}, {i: immediate} =>
        (1`2 @ 0b1 @ d`5) @ 0x02 @ a @ 0x00 @ i

    nor {d: register}, {a: register}, {i: immediate} =>
        (1`2 @ 0b1 @ d`5) @ 0x03 @ a @ 0x00 @ i

    add {d: register}, {a: register}, {i: immediate} =>
        (1`2 @ 0b1 @ d`5) @ 0x04 @ a @ 0x00 @ i

    sub {d: register}, {a: register}, {i: immediate} =>
        (1`2 @ 0b1 @ d`5) @ 0x05 @ a @ 0x00 @ i

    xor {d: register}, {a: register}, {i: immediate} =>
        (1`2 @ 0b1 @ d`5) @ 0x06 @ a @ 0x00 @ i

    lsl {d: register}, {a: register}, {i: immediate} =>
        (1`2 @ 0b1 @ d`5) @ 0x07 @ a @ 0x00 @ i

    lsr {d: register}, {a: register}, {i: immediate} =>
        (1`2 @ 0b1 @ d`5) @ 0x08 @ a @ 0x00 @ i

    mul {d: register}, {a: register}, {i: immediate} =>
        (1`2 @ 0b1 @ d`5) @ 0x09 @ a @ 0x00 @ i

    cmp {d: register}, {a: register}, {i: immediate} =>
        (1`2 @ 0b1 @ d`5) @ 0x0a @ a @ 0x00 @ i

    div {d: register}, {a: register}, {i: immediate} =>
        (1`2 @ 0b1 @ d`5) @ 0x0b @ a @ 0x00 @ i

    rem {d: register}, {a: register}, {i: immediate} =>
        (1`2 @ 0b1 @ d`5) @ 0x0c @ a @ 0x00 @ i

    ; extra

    inc {a: register} => asm { add {a}, {a}, 1 }
    dec {a: register} => asm { sub {a}, {a}, 1 }

    mov {d: register}, {a: register}  => asm { or {d}, {a}, zr }
    mov {d: register}, {i: immediate} => asm { or {d}, zr, {i} }

    neg {a: register}, {b: register} => asm { sub {a}, zr, {b} }
    neg {a: register}, {b: immediate} => asm { sub {a}, zr, {b} }

    not {a: register}, {b: register} => asm { nor {a}, zr, {b} }
    not {a: register}, {b: immediate} => asm { nor {a}, zr, {b} }

    ;
    ; cond
    ;

    jmp {d: register}  => (2`2 @ 0b0 @ d`5) @ 0x00 @ 0x00 @ 0x00
    jmp {i: immediate} => (2`2 @ 0b1 @ 0`5) @ 0x00 @ 0x00 @ 0x00 @ i
    je {a: register}, {i: immediate}  => (2`2 @ 0b1 @ 0`5) @ 0x01 @ a @ 0x00 @ i
    jne {a: register}, {i: immediate} => (2`2 @ 0b1 @ 0`5) @ 0x02 @ a @ 0x00 @ i
    jl {a: register}, {i: immediate}  => (2`2 @ 0b1 @ 0`5) @ 0x03 @ a @ 0x00 @ i
    jge {a: register}, {i: immediate} => (2`2 @ 0b1 @ 0`5) @ 0x04 @ a @ 0x00 @ i
    jle {a: register}, {i: immediate} => (2`2 @ 0b1 @ 0`5) @ 0x05 @ a @ 0x00 @ i
    jg {a: register}, {i: immediate}  => (2`2 @ 0b1 @ 0`5) @ 0x06 @ a @ 0x00 @ i
    jb {a: register}, {i: immediate}  => (2`2 @ 0b1 @ 0`5) @ 0x07 @ a @ 0x00 @ i
    jae {a: register}, {i: immediate} => (2`2 @ 0b1 @ 0`5) @ 0x08 @ a @ 0x00 @ i
    jbe {a: register}, {i: immediate} => (2`2 @ 0b1 @ 0`5) @ 0x09 @ a @ 0x00 @ i
    ja {a: register}, {i: immediate}  => (2`2 @ 0b1 @ 0`5) @ 0x0a @ a @ 0x00 @ i

    ; stack

    ; push {a: register} => asm {
    ;     sub sp, sp, 2
    ;     str.w mem[sp], {a}
    ; }
    push {a: register} => (3`2 @ 0b0 @ 0`5) @ 0x00 @ a @ 0x00

    ; pop {a: register} => asm {
    ;     ld.w {a}, mem[sp]
    ;     add sp, sp, 2
    ; }
    pop {a: register} => (3`2 @ 0b0 @ 0`5) @ 0x01 @ a @ 0x00

    ; call {i: immediate} => asm {
    ;     rdpc flags
    ;     add flags, flags, 20
    ;     sub sp, sp, 2
    ;     str.w mem[sp], flags
    ;     jmp {i}
    ; }
    call {a: register}  => (3`2 @ 0b0 @ 0`5) @ 0x02 @ a @ 0x00
    call {i: immediate} => (3`2 @ 0b1 @ 0`5) @ 0x02 @ 0x00 @ 0x00 @ i

    ; ret => asm {
    ;     ld.w flags, mem[sp]
    ;     add sp, sp, 2
    ;     jmp flags
    ; }
    ret => (3`2 @ 0b0 @ 0`5) @ 0x03 @ 0x00 @ 0x00
}
