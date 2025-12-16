# Brainfuck to C Compilation: Process
This section describes the process by which Brainfuck source code is translated into equivalent C code under the strict Brainfuck specification defined above. The goal of this compilation process is to produce C code that, when compiled and executed, exhibits behavior observationally equivalent to the execution of the original Brainfuck program.

## Filtering
The first step of the translation process is a one-pass filtering stage.

The Brainfuck program is first filtered through to remove any non-Brainfuck characters. Such characters are treated as comments in the program. The result of this is a linear sequence of Brainfuck characters.

## Lexing and Parsing
The second step of the translation process is the lexing and parsing stage.

The Brainfuck program is parsed through step-by-step, where each character is lowered into an intermediate representation suitable for code generation. In this stage, we also make sure that all instances of brakcets are properly matched and balanced. There is an optional flag that can be set to disable this process. We ONLY recommend disabling this flag if you are 100% sure that all the BF files you write are well-formed, otherwise it should be enabled at all times.

## Code Generation Optimization
The third step of the translation process is the optimization stage.

This step preserves program semantics while simplifying the generation of readable and efficient C code. All of our optimizations are applied to the code being generated. To learn more about the specific optimizations applied in this product, consult Section 3.

## Code Emitting
The final step involves emitting C Code that directly mirrors the Brainfuck model and the code generated.

The generated C Code defines:
- A contiguous array of 200,000 bytes to represent the Brainfuck tape
- A movable pointer into this array
- All elements initialized to zero

Each Brainfuck instruction is translated into an equivalent C construct:
- Pointer movement is translated into pointer arithmetic
- Arithmetic operations are translated into byte arithmetic with modulo-256 behavior
- Input and output operations use standard C I/O functions
- Brainfuck loops are translated into C while loops that test the current cell

The generated C code is considered functionally correct provided that:
- The data pointer never moves outside the bounds of the allocated tape
- Input and output behavior conforms to the specified semantics
- Arithmetic overflow wraps modulo 256
Programs that violate these assumptions may exhibit undefined behavior in the generated C code, consistent with the strict Brainfuck specification used by this project.