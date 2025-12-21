# C to Brainfuck Compilation: Implementation

Brief implementation guide for C to Brainfuck compiler.

## Table of Contents

### Part 1: Fundamentals
1. [EBNF for C Subset](#ebnf-for-c-subset)
2. [Variables](#variables)
3. [I/O Operations](#io-operations)

### Part 2: Expressions & Operators
4. [Arithmetic Operations](#arithmetic-operations)
5. [Comparison Operations](#comparison-operations)
6. [Logical Operators](#logical-operators)
7. [Bitwise Operations](#bitwise-operations)
8. [sizeof Operator](#sizeof-operator)

### Part 3: Control Flow
9. [Conditionals](#conditionals)
10. [Ternary Operator](#ternary-operator)
11. [Switch Statements](#switch-statements)
12. [While Loops](#while-loops)
13. [Do-While Loops](#do-while-loops)
14. [For Loops](#for-loops)
15. [Break and Continue](#break-and-continue)

### Part 4: Data Structures
16. [Arrays](#arrays)
17. [Strings](#strings)
18. [Pointers](#pointers)
19. [Structs](#structs)
20. [Enumerations (enum)](#enumerations-enum)
21. [Type Definitions (typedef)](#type-definitions-typedef)

### Part 5: Advanced Features
22. [Functions](#functions)
23. [Dynamic Memory](#dynamic-memory)

### Appendices
- [Appendix A: Common Brainfuck Idioms and Patterns](#appendix-a-common-brainfuck-idioms-and-patterns)
- [Appendix B: Optimization Opportunities](#appendix-b-optimization-opportunities)

---

# Part 1: Fundamentals

## EBNF for C Subset

```C

EBNF Syntax:
defintion     =
concatenation ,
termination   ;
alternation   |
optional      []
repetition    {}
grouping      ()
string        ""
comment       (**)
regex         //

-------------------

 ident  = /[A-Za-z][A-Za-z0-9]*/ ;
nat    = /[0-9]+/ ;
atom   = ident | nat | "getchar()" ;
term   = atom , [ ( "*" | "/" ) , atom ] ;
expr   = "!" , term
       | term , [ ( ( "==" | ">" | "<" | "+" ) , term )
                | "++"
                | "--"
                ] ;
stmt   = "char" , ident , ";"
       | ident , "=" , expr , ";"
       | "if" , "(" , expr , ")" , "{" , block , "}"
       | "if" , "(" , expr , ")" , "{" , block , "}" ,
         "else" , "{" , block , "}"
       | "while" , "(" , expr , ")" , "{" , block , "}"
       | "for" , "(" , block , ";" , expr , ";" , block , ")" ,
         "{" , block , "}"
       | "putchar" , "(" , atom , ")" , ";"
       | expr , ";" ;
block  = stmt , { stmt } ;
```

## Variables

**Implementation:** Map C variables to BF tape cells. Maintain symbol table with variable names and their tape addresses. 

## I/O Operations

**Implementation:** `getchar()` uses `,` command. `putchar()` uses `.` command.

---

# Part 2: Expressions & Operators

## Arithmetic Operations

**Implementation:** Use accumulator (AX) register pattern. Addition/subtraction via increment/decrement loops. Multiplication via nested loops. Division via repeated subtraction. Use temporary cells for intermediate values.

## Comparison Operations

**Implementation:** Compute difference `a - b` in temp cell.

## Logical Operators

**Implementation:** 

## Bitwise Operations

**Implementation:** 

## sizeof Operator

**Implementation:** 

---

# Part 3: Control Flow

## Conditionals

**Implementation:**

## Ternary Operator

**Implementation:** Desugar to if-else statement: `c ? a : b` becomes `if(c) {temp=a;} else {temp=b;}`. 

## Switch Statements

**Implementation:** Convert to if-else chain comparing expression against each case. For dense cases, optionally use jump table with computed addresses.

## While Loops

**Implementation:** Evaluate condition.

## Do-While Loops

**Implementation:** Execute block once unconditionally. Then evaluate condition and use BF `[...]` loop for subsequent iterations.

## For Loops

**Implementation:** Desugar to while loop: execute init once, then while(condition) { body; update; }.

## Break and Continue

**Implementation:** 

---

# Part 4: Data Structures

## Arrays

**Implementation:**

## Strings

**Implementation:** 

## Pointers

**Implementation:** 

## Structs

**Implementation:** 

## Enumerations (enum)

**Implementation:** 
## Type Definitions (typedef)

**Implementation:** 
---

# Part 5: Advanced Features

## Functions

**Implementation:** 

## Dynamic Memory

**Implementation:** 

---

# Appendix A: Common Brainfuck Idioms and Patterns

**Implementation:** Document reusable BF code patterns:
- Clear cell: `[-]`
- Move value: `[->+<]`
- Copy value: `[->+>+<<]>>[-<<+>>]<<`
- Boolean NOT: `>+<[>-<[-]]>`
- If-then: `[->+<]>[...[-]]`
- Multiply: nested loops with accumulator
- Set constant: `[-]` then `+` n times, or use multiplication for large values
