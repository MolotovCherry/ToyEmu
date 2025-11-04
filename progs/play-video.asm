_start:
    mov t0, 800
    mov t1, 600
    mov t2, 30
    gfx

    call loop

    gfx ; stop graphics

    hlt

loop:
    mov s8, 0x2800     ; file beginning
loop2:
    mov s1, 0x80000000 ; vram addr

    draw

    color_loop:
        ; Counter of many pixels of color to use
        ld t1, [s8]
        add s8, s8, 4

        mov t0, 0xffffffff
        je t1, t0, loop2

        ld.b t2, [s8] ; color
        add s8, s8, 1 ; for next color_loop run

        ; format is 0RGB
        mul t2, t2, 0x00010101

        mul t1, t1, 4
        smem [s1], t1, t2
        add s1, s1, t1

        jmp color_loop
