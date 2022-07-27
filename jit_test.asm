	.section	__TEXT,__text,regular,pure_instructions
	.build_version macos, 12, 0
	.globl	_executecontract
	.p2align	2
_executecontract:
	.cfi_startproc
	ldr	x8, [x0]
	str	xzr, [x8, #56]
	movi.16b	v0, #0
	stur	q0, [x8, #40]
	mov	w9, #1
	stp	xzr, x9, [x8, #24]
	stur	q0, [x8, #8]
	mov	w9, #2
	str	x9, [x8]
	mov	x0, #0
	ret
	.cfi_endproc

.subsections_via_symbols
