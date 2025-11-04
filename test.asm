_start:
    mov t0, 800
    mov t1, 600
    mov t3, 30
    gfx

    call loop

    gfx ; stop graphics

    hlt

loop:
    mov s8, 0x2800     ; file beginning
loop2:
    mov t6, 480000
    mov s1, 0x80000000 ; vram addr

    draw

    color_loop:
        ; Counter of many pixels of color to use
        ld t0, [s8]
        add s8, s8, 4
        mov t1, t0 ; backup the count

        ld.b t2, [s8] ; color
        add s8, s8, 1 ; for next color_loop run

        ; format is 0RGB
        mul t2, t2, 0x00010101

        jmp pixel_write
    cont:

        sub t6, t6, t1
        ja t6, zr, color_loop

        jmp loop2

    pixel_write:
        str [s1], t2
        add s1, s1, 4

        sub t0, t0, 1
        ja t0, zr, pixel_write

        jmp cont
