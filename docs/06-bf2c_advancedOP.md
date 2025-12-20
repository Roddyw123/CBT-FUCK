# Brainfuck to C Compilation: Experimental Optimizations
This document describes all **experimental** optimizations done by the compiler. These go beyond standard techniques.

Here is a table of contents for all of the current optimizations:
1. [Pattern-Based Macro Expansions](#pattern-based-macro-expansions)
## Pattern-Based Macro Expansions
We want to recognize high-level algorithmic patterns such as divison/modulo/comparison/multiplication and replace entire loop sequences involving these patterns with equivaleent C operations.