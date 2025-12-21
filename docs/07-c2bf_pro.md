# C to Brainfuck Compilation: Process
This section describes the process by which C source code is translated into equivalent Brainfuck code under the strict Brainfuck specification defined above. The goal of this compilation process is to produce C code that, when compiled and executed, exhibits behavior observationally equivalent to the execution of the original C program.

## Components

1. Tokenizer
2. Parser
3. Emitter
4. Optimizations

## Part 1: Tokenizer
Trivial

## Part 2: Parser
Trivial

## Part 3: Emitter

We divide the BF tape into 3 sections, namely:
1. Registers
2. Instruction Memory
3. Stack

### A. Registers
We store the Instruction Pointer (IP), Stack Pointer (SP), Base Pointer (BP), Accumulator (AX), and 3 {to be determined} miscellaneous registers for arithmetic.
We also store the address of the last instruction.
The location of these registers should be stored as macros/ a lookup table within c2bf.

### B. Instruction Memory
We store an opcode and up to three operands within the instruction memory. 
We will use a BF assembly based on a reduced version of RISC-V.

### C. The Stack
For the sake of simplicity, let us construct the stack counting up since the heap is not yet implemented.

We know the size of the instruction memory at compile time. Global variables are allocated addresses with constant offset from the base of the stack.

All other variables are associated with a function. We can assign an ID to every function declaration. 
When a function is called, its arguments are pushed onto the stack. The stack and base pointer are updated.
Variable declarations in the function are allocated cells after the arguments. We do not use registers for argument passing for simplicity.

When the function returns, the result is passed to the accumulator.

### D. Heap 

TODO("Implement after stack has been done")

### E. Emitting the Program
We emit a bf program that goes through the instruction memory. 

Every loop, we:
1. Check if the program is finished running (IP == last instruction address);
2. Traverse to the address of the IP;
3. Match the opcode with our BF assembly. Early return if invalid;
4. Perform the instruction.

Each C statement would be translated into corresponding BF assembly code (BFA). 
It can be useful to have an tag to generate this intermediate state with a custom file extension i.e. .bfa

The generated BF file would contain the translations of all BFA to BF.

## Part 4: Optimizations

## Tags
We would need the following tags for c2bf
1. Only verify that the input file matches our EBNF
2. Only generate BFA
3. BFA optimizations
4. BF optimizations


