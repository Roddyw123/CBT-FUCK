# CBT-FUCK Documentation

**A bidirectional C ↔ Brainfuck transpiler built with a focus on simplicity, correctness, and extensibility.**

CBT-FUCK is a hobbyist project motivated by unfamiliar compilation targets and deliberately hostile semantics. This project is far from practical, and is instead defined by a pursuit of challenge, experimentation, and understanding how far low-level translation can be pushed.

This is a complete rewrite of CBT, another bidirectional C ↔ Brainfuck transpiler built solely on C.

**Repository:** [CBT-FUCK on GitHub](https://github.com/Roddyw123/CBT-FUCK)

This project is ongoing.

---

## Getting Started

- [**Motivation**](00-motivation) - Why this project exists
- [**Introduction**](01-intro) - Understanding Brainfuck and its specification

---

## Brainfuck to C Compilation

Learn how Brainfuck source code is translated into equivalent C code:

### Core Process
- [**Compilation Process**](02-bf2c_pro) - Overview of filtering, parsing, optimization, and code emission

### Optimizations
- [**Optimization Level 1**](03-bf2c_op1) - Local optimizations
- [**Optimization Level 2**](04-bf2c_op2) - Analysis passes for global optimizations
- [**Optimization Level 3**](05-bf2c_op3) - Global optimizations
- [**Advanced Optimizations**](06-bf2c_advancedOP) - Experimental and Cutting-edge optimization techniques

---

## C to Brainfuck Compilation

Explore the reverse process of translating C code into Brainfuck:

- [**C to Brainfuck Process**](07-c2bf_pro) - Overview of the C to Brainfuck compilation pipeline
- [**Implementation Details**](08-c2bf_imp) - Technical implementation and design decisions

---

## Additional Resources

- [**Conclusion**](09-conc) - Final thoughts and future directions
- [**Credits**](CREDITS) - Acknowledgments and references

---
