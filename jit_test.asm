	.section	__TEXT,__text,regular,pure_instructions
	.build_version macos, 12, 0
	.globl	_executecontract
	.p2align	2
_executecontract:
	.cfi_startproc
	ldr	x8, [x0]
	mov	x9, #48938
	movk	x9, #63706, lsl #16
	movk	x9, #30108, lsl #32
	movk	x9, #16806, lsl #48
	mov	x10, #53013
	movk	x10, #54344, lsl #16
	movk	x10, #24567, lsl #32
	movk	x10, #56862, lsl #48
	stp	x10, x9, [x8, #48]
	mov	x9, #27962
	movk	x9, #18171, lsl #16
	movk	x9, #4748, lsl #32
	movk	x9, #52969, lsl #48
	mov	x10, #63170
	movk	x10, #45911, lsl #16
	movk	x10, #21054, lsl #32
	movk	x10, #26036, lsl #48
	stp	x10, x9, [x8, #32]
	mov	x9, #22393
	movk	x9, #49813, lsl #16
	movk	x9, #10560, lsl #32
	movk	x9, #63592, lsl #48
	mov	x10, #8288
	movk	x10, #38807, lsl #16
	movk	x10, #50367, lsl #32
	movk	x10, #44575, lsl #48
	stp	x10, x9, [x8, #16]
	mov	x9, #44939
	movk	x9, #28984, lsl #16
	movk	x9, #63559, lsl #32
	movk	x9, #45102, lsl #48
	mov	x10, #1514
	movk	x10, #38055, lsl #16
	movk	x10, #30731, lsl #32
	movk	x10, #17285, lsl #48
	stp	x10, x9, [x8]
	mov	x0, #0
	ret
	.cfi_endproc

.subsections_via_symbols
