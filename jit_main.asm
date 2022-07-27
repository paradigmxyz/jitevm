	.section	__TEXT,__text,regular,pure_instructions
	.build_version macos, 12, 0
	.globl	_executecontract
	.p2align	2
_executecontract:
	.cfi_startproc
	ldr	x8, [x0]
	movi.16b	v0, #0
	stur	q0, [x8, #24]
	stur	q0, [x8, #8]
	mov	w9, #5998
	str	x9, [x8]
	stur	q0, [x8, #40]
	str	xzr, [x8, #88]
	stur	q0, [x8, #72]
	mov	w0, #1
	stp	xzr, x0, [x8, #56]
	add	x8, x8, #96
	mov	w9, #28
	mov	x10, #-1
	mov	w11, #7
LBB0_1:
	ldp	q1, q0, [x8, #-96]
	stp	q1, q0, [x8]
	ldp	x12, x13, [x8, #16]
	ldp	x14, x15, [x8]
	orr	x13, x15, x13
	orr	x12, x14, x12
	orr	x12, x12, x13
	cmp	x12, #0
	csel	x12, xzr, x0, ne
	csel	x13, xzr, xzr, ne
	stp	x12, x13, [x8]
	stp	x13, x13, [x8, #16]
	stp	xzr, xzr, [x8, #48]
	stp	x9, xzr, [x8, #32]
	orr	x12, x12, x13
	orr	x13, x13, x13
	orr	x12, x12, x13
	cbz	x12, LBB0_4
	eor	x12, x9, #0x1c
	cbz	x12, LBB0_6
	eor	x12, x9, #0x7
	cbz	x12, LBB0_1
	b	LBB0_7
LBB0_4:
	ldp	q1, q0, [x8, #-64]
	stp	q1, q0, [x8]
	ldp	q0, q1, [x8, #-32]
	ldur	q2, [x8, #-32]
	stp	q0, q1, [x8, #32]
	ldp	x13, x12, [x8, #48]
	ldp	x15, x14, [x8, #32]
	ldp	x17, x16, [x8, #16]
	ldp	x2, x1, [x8]
	adds	x15, x15, x2
	adcs	x14, x14, x1
	adcs	x13, x13, x17
	adcs	x12, x12, x16
	ldp	x17, x16, [x8, #-16]
	stp	x17, x16, [x8, #-48]
	stur	q2, [x8, #-64]
	ldp	x17, x16, [x8, #-96]
	ldp	x1, x2, [x8, #-80]
	stp	x1, x2, [x8, #-16]
	stp	x17, x16, [x8, #-32]
	stp	x15, x14, [x8, #-96]
	stp	x13, x12, [x8, #-80]
	stp	x17, x16, [x8]
	stp	x1, x2, [x8, #16]
	stp	xzr, xzr, [x8, #-16]
	stp	x0, xzr, [x8, #-32]
	mov	x12, x2
	mov	x13, x1
	mov	x14, x16
	mov	x15, x17
	subs	x15, x15, #1
	adcs	x14, x14, x10
	adcs	x13, x13, x10
	adcs	x12, x12, x10
	ldur	q0, [x8, #-96]
	ldp	x16, x17, [x8, #-80]
	stp	x16, x17, [x8, #-16]
	stur	q0, [x8, #-32]
	stp	x15, x14, [x8, #-96]
	stp	x13, x12, [x8, #-80]
	stp	xzr, xzr, [x8, #16]
	stp	x11, xzr, [x8]
	cbnz	wzr, LBB0_6
	cmp	x11, #7
	b.eq	LBB0_1
	b	LBB0_7
LBB0_6:
	mov	x0, #0
	ldur	q0, [x8, #-32]
	ldp	x10, x9, [x8, #-16]
	ldur	q1, [x8, #-96]
	ldp	x12, x11, [x8, #-80]
	stp	x12, x11, [x8, #-16]
	stur	q1, [x8, #-32]
	stp	x10, x9, [x8, #-80]
	stur	q0, [x8, #-96]
LBB0_7:
	ret
	.cfi_endproc

.subsections_via_symbols
