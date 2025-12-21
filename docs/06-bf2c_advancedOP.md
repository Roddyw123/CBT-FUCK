# Brainfuck to C Compilation: Experimental Optimizations
This document describes all **experimental** optimizations done by the compiler. These go beyond standard techniques.

Here is a table of contents for all of the current optimizations:
1. [Pattern-Based Macro Expansions](#pattern-based-macro-expansions)
2. [Loop Invariant Code Motion](#loop-invariant-code-motion)
## Pattern-Based Macro Expansions
We want to recognize high-level algorithmic patterns such as divison/modulo/comparison/multiplication and replace entire loop sequences involving these patterns with equivaleent C operations.

## Loop Invariant Code Motion
Moving code outside of the body of a loop without affecting the semantics of the program. 