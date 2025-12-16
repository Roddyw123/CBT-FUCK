# C to Brainfuck Compiler

## Introduction


## Components

1. Tokenizer
2. Parser
3. Emitter
4. Variable Table

## Part 1: Compile-time Allocated Variables



### Arrays!~

Now how do we construct Arrays in Brainfuck?

Problem:
TODO

We save the start of the array in the variable table.


On the Brainfuck Memory tape, we have 3 buffer slots for every element we want.
For example: for the array `[1, 5, 7]`, we have:

`[0] [0] [0] [1] [0] [0] [0] [5] [0] [0] [7]`

` ↑ variable table pointer`

TODO











Part 1 is the extent of the original CBT (C Project Extension)

## Part 2: Building the Stack

## Part 3: Building the Heap


