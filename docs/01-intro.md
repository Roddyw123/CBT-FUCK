# Introduction
Brainfuck is an imperative, esoteric programming language that is Turing-Complete, created in 1993 by a Swiss Student called Urban Müller. Brainfuck is designed to be heavily minimalistic and only supports 8 single character instructions, making it highly impractical. Brainfuck is considered a Turing Tarpit; a type of language that can write any program but is ultimately impractical due to any lack of abstraction.

Brainfuck uses a simple machine model consisting of a one-dimensional array that is bounded below at a size of at least 30,000 byte cells (8 bit memory blocks) initialized to zero (non-negative integers), a moveable data pointer initialized to the leftmost byte of the array that only points to one cell and only operates on the current cell, and also consists of a program plus an instruction pointer.

Below is the canonical Extended Backus-Naur Form for Brainfuck:
- program = { instruction } ;
- instruction = ">" | "<" | "+" | "-" | "." | "," | loop ;
- loop = "[" program "]" ; 

Note that programs with unmatched brackets are syntax errors. Brackets "[", "]" must be properly nested and balanced.

## Brainfuck Instructions
Below are the descriptions of each Brainfuck symbol and their equivalent to C.

| Symbol | Description | C Equivalent |
|--------|-------------|--------------|
| `>` | Increment the data pointer (move right one cell) | `ptr++;` |
| `<` | Decrement the data pointer (move left one cell) | `ptr--;` |
| `+` | Increment the byte at the data pointer | `(*ptr)++;` |
| `-` | Decrement the byte at the data pointer | `(*ptr)--;` |
| `.` | Output the byte at the data pointer | `putchar(*ptr);` |
| `,` | Input one byte and store it at the data pointer | `*ptr = getchar();` |
| `[` | If the byte at the data pointer is zero, jump forward to the matching `]` | `while (*ptr) {` |
| `]` | If the byte at the data pointer is nonzero, jump back to the matching `[` | `}` |

## Strict Specification of Brainfuck used
In this project, the following will be assumed about the Brainfuck language:
- The byte array will consist of 200,000 byte elements.
- All byte array elements will be non-negative integers.
- All arithmetic is performed modulo 256.