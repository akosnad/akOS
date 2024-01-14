.section .text
.globl context_switch

.align 64
.code64

# context_switch(current: *mut ThreadInfo, next: *mut ThreadInfo)
context_switch:
  push rbx
  push rbp
  push r12
  push r13
  push r14
  push r15

  mov rax, cr2
  push rax
  pushfq

  mov [rdi], rsp
  mov rsp, [rsi]
  cli
  popfq
  pop rax
  mov cr2, rax

  pop r15
  pop r14
  pop r13
  pop r12
  pop rbp
  pop rbx

  ret
