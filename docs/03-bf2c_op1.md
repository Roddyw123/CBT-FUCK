# Brainfuck to C Compilation: Local Optimization

The translation from Brainfuck to C is considered trivial due to the semantic closeness of these languages. Therefore, the primary focus of CBT-FUCK's compilation pipeline is the generation of highly optimized C code through pattern recognition and algebraic transformation.

The following 3 markdown files descrube the optimization passes applied during compilation, their mathematical foundations, and the correctness guarantees they provide.

This document describes all **local** optimizations done by the compiler, which involves pattern matching and only uses local information.

Here is a table of contents for all of the current optimizations:

1. [Instruction Coalescing and Cancellation](#instruction-coalescing-and-cancellation)
2. [Zero Loops](#zero-loops)
3. [Scan Loops](#scan-loops)
4. [Multiplication Loops](#multiplication-loops)

## Intermediate Representation

Below is the IR that we have decided on, in Backus-Naur Form:

```
Prog ::= Stmt*

Stmt ::= Add(delta: i32)
       | Move(distance: i32)
       | Output(delta: i32)
       | Input(delta: i32)
       | Input
       | Loop(body: Prog)
       | ZeroLoop
       | ScanLoop(direction: i32)
       | MultiplicationLoop(decrement: u8, effects: (offset: i32, factor: i32)*)
```

### IR Node Semantics

- **Add(delta)**: Add `delta` to the current cell (delta can be negative for subtraction). Result of coalescing consecutive `+` and `-` operations.

- **Move(distance)**: Move the data pointer by `distance` positions (distance can be negative for leftward movement). Result of coalescing consecutive `>` and `<` operations.

- **Output(delta)**: Output the value of the current cell `delta` times. Result of coalescing `delta` consecutive `.` operations.

- **Input(delta)**: Read `delta` bytes from input, storing only the final byte in the current cell. Result of coalescing `delta` consecutive `,` operations. The first `delta-1` bytes are consumed from stdin but discarded.

- **Input**: Read a single byte from input into the current cell (corresponds to `,` in Brainfuck). Used when no coalescing occurs.

- **Loop(body)**: Execute the program `body` repeatedly while the current cell is non-zero. Represents a general `[...]` loop that hasn't been optimized into a more specific form.

- **ZeroLoop**: Set the current cell to zero in O(1) time. Recognized from patterns `[-]` or `[+]`.

- **ScanLoop(direction)**: Move the pointer in the given direction until reaching a zero cell. `direction` is +1 for right scan `[>]`, -1 for left scan `[<]`. More generally, `direction` indicates the amount to move per iteration (though typically ±1).

- **MultiplicationLoop(decrement, effects)**: An optimized linear loop that:
  - Decrements the current cell by `decrement` per iteration (where `decrement` is odd to guarantee termination)
  - Applies scaled changes to cells at various offsets
  - `effects` is a list of `(offset, factor)` pairs, where each pair means "add `factor * (initial_value / decrement)` to the cell at `offset`"
  - Executes in O(1) time instead of O(n) 


---

## Instruction Coalescing and Cancellation

The simplest optimizations performed by CBT-FUCK are **instruction coalescing** and **cancellation**. These operate on maximal consecutive runs of Brainfuck instructions drawn from either:
- The **arithmetic set**: `{+, -}`
- The **pointer-movement set**: `{>, <}`
- The **I/O set**: `{., ,}`

### Algorithm

For each maximal run of instructions from the same set, the compiler computes the net effect:

**Arithmetic runs:**
- `+` contributes +1 to the current cell value
- `-` contributes -1 to the current cell value

**Pointer-movement runs:**
- `>` contributes +1 to the data pointer position
- `<` contributes -1 to the data pointer position

**I/O runs:**
- `.` outputs the current cell value once
- `,` reads one byte from input, storing it in the current cell

Note: Multiple consecutive `,` operations each overwrite the cell, so only the final input value remains. However, all bytes must still be consumed from the input stream (side effect).

### Transformation

The entire run is replaced with a single combined operation:
- **Add(k)**: net arithmetic effect of k
- **Move(n)**: net pointer movement of n
- **Input(n)**: net inputs of n
- **Output(n)**: net outputs of n

If the net effect is zero, the run is **removed entirely** (cancellation). Note: This only applies to arithmetic and pointer-movement operations; I/O operations are never cancelled due to their observable side effects.

### Examples

```bf
+++--+     →  Add(2)
>><>>      →  Move(3)
><><       →  (deleted)
--+++--    →  Add(-1)
.....      →  Output(5)
,,,        →  Input(3)
```

### Correctness

**For arithmetic and pointer movement:**

This optimization is trivially correct because addition and pointer arithmetic are both associative and commutative operations over ℤ/256ℤ and ℤ respectively. The final state is identical whether operations are applied individually or as an aggregate:

```
(+1) + (+1) + (-1) + (+1) = +2
(>1) + (>1) + (<1) + (>1) + (>1) = >3
```

**For I/O operations:**

I/O coalescing requires more careful analysis due to side effects:

1. **Output operations (`.`)**:
   Multiple consecutive outputs can be coalesced because each operation independently writes the current cell value to the output stream. Outputting the value n times sequentially is equivalent to a loop that outputs n times:
   ```c
   // Original: . . . . .
   putchar(*ptr); putchar(*ptr); putchar(*ptr); putchar(*ptr); putchar(*ptr);

   // Coalesced: Output(5)
   for (int i = 0; i < 5; i++) putchar(*ptr);

   // Further optimization for large n: buffered write
   // (Only beneficial when n is large enough to amortize overhead)
   if (n > 8) {  // Threshold determined empirically
       char buf[n];  // VLA for moderate n, or heap allocation for very large n
       memset(buf, *ptr, n);
       fwrite(buf, 1, n, stdout);
   } else {
       for (int i = 0; i < n; i++) putchar(*ptr);
   }
   ```
   Both produce identical output: the same cell value written to stdout exactly 5 times. ✓

   **Note:** The buffering optimization reduces function call overhead but may not improve performance for small n due to setup cost. Additionally, `putchar` is already buffered by the C standard library, so this optimization primarily benefits scenarios with very large n or when fine-grained control over system calls is needed.

2. **Input operations (`,`)**:
   Multiple consecutive inputs can be coalesced with the understanding that:
   - Each `,` reads one byte from stdin (consumes from input stream)
   - Each `,` overwrites the current cell with the newly read value
   - Only the last value written remains in the cell

   For n consecutive input operations, the semantics are:
   ```c
   // Original: , , ,
   *ptr = getchar(); *ptr = getchar(); *ptr = getchar();

   // Coalesced: Input(3) - basic loop
   for (int i = 0; i < 3; i++) *ptr = getchar();

   // Optimized: Input(3) - explicit discard + final read
   for (int i = 1; i < 3; i++) getchar();  // Discard first n-1 bytes
   *ptr = getchar();  // Keep the nth byte

   // Alternative for small compile-time known n: fully unroll
   getchar(); getchar(); *ptr = getchar();
   ```
   All variants consume exactly 3 bytes from the input stream, and the cell contains the 3rd byte read. The first two bytes are discarded (overwritten), but the critical side effect—consuming bytes from stdin—is preserved. ✓

   The optimized version saves n-1 assignment operations and makes the intent clearer. For small known n (e.g., n ≤ 4), full loop unrolling eliminates loop overhead entirely.

   **Important:** We cannot optimize this to a single `*ptr = getchar()` even though only the final byte is stored. We must consume all n bytes from stdin, otherwise subsequent input operations would read the wrong bytes.

   **Example:** With input stream `ABC`:
   - `,,,` consumes A, B, C (stores C)
   - A single `,` consumes only A (stores A)
   - These produce different program states! The next `,` would read D in the first case, but B in the second.

**Cancellation:**

For arithmetic and pointer movement, if the net effect is zero, the entire run can be safely removed. However, **I/O operations cannot be cancelled** even if they appear redundant, because:
- Output operations have observable side effects (writing to stdout)
- Input operations have side effects (consuming from stdin)

Therefore, `Output(n)` and `Input(n)` are only generated for n ≥ 1, never removed by cancellation.

---

## Zero Loops

A **zero loop** is a Brainfuck loop pattern whose sole effect is to set the current cell to zero. These loops appear in two canonical forms:

```bf
[-]    # Decrement until zero
[+]    # Increment until zero
```

### Mathematical Foundation

Under the strict Brainfuck specification used by this project, all arithmetic is performed **modulo 256** (8-bit unsigned integers). As a result:

- Starting from any value x ∈ [0, 255]
- Repeatedly applying +1 or -1 modulo 256
- The sequence is guaranteed to eventually reach 0

This follows from the fact that (ℤ/256ℤ, +) is a cyclic group, and both +1 and -1 generate the entire group.

### Equivalence

The following equivalence holds under modulo 256 arithmetic:

```text
[-] ≡ [+] ≡ *ptr = 0;
```

### Transformation

When the compiler encounters a loop whose body consists solely of a single increment or decrement instruction (after coalescing), it replaces the loop with a direct assignment:

**Before:**
```c
while (*ptr) {
    (*ptr)--;  // or (*ptr)++;
}
```

**After:**
```c
*ptr = 0;
```

### Performance Impact

This transformation:
- Eliminates loop dispatch overhead entirely
- Guarantees **O(1) constant-time** execution (vs. O(n) where n ≤ 255)
- Reduces code size
- Preserves exact Brainfuck semantics

### Safety Conditions

This optimization is valid only if:
- The loop body contains **no pointer movement** (`>` or `<`)
- The loop body contains **no I/O operations** (`,` or `.`)
- Arithmetic is performed **modulo a finite cell width** (guaranteed by uint8_t)

---

## Scan Loops

A **scan loop** is a Brainfuck pattern that searches for the nearest zero cell in a given direction. These loops move the data pointer until it points to a cell containing zero.

### Canonical Forms

**Right scan** (search forward):
```bf
[>]
```

**Left scan** (search backward):
```bf
[<]
```

### Semantics

A scan loop has the following properties:
- The loop body contains **only** a single pointer movement instruction
- No arithmetic or I/O operations occur
- The loop terminates when `*ptr == 0`

The effect is to find the closest zero cell in the specified direction, including the current cell if it already contains zero.

### Transformation

**Right scan `[>]`:**
```c
while (*ptr) ptr++;
```

**Left scan `[<]`:**
```c
while (*ptr) ptr--;
```

### Performance Impact

This optimization:
- Eliminates loop body overhead (no need to check loop body instructions)
- Produces idiomatic C code that compilers can optimize well
- Improves code readability
- Has the same time complexity as the original (O(k) where k is distance to zero)

### Rationale

While this transformation does not reduce asymptotic complexity, it provides:
- **Reduced constant factors** by eliminating loop dispatch overhead
- **Better readability** through idiomatic single-line C
- **Compiler optimization opportunities** (e.g., vectorization, prefetching)

---

## Multiplication Loops

A **multiplication loop** (also called a **linear loop** or **transfer loop**) is a Brainfuck loop whose net effect is a linear transformation of tape cells. When optimized, the loop can be replaced by a fixed number of arithmetic operations instead of iteration proportional to the cell value.

Informally, such a loop:
1. Consumes the value of the current cell (the **control cell**)
2. Distributes scaled copies of that value to other cells
3. Leaves the control cell as zero upon termination

A **copy loop** is a special case where all scaling factors are ±1.

### Recognition Criteria

A loop qualifies as a multiplication loop if and only if:

1. **Zero net pointer movement**: The pointer ends at the same position it started
2. **Pure arithmetic operations**: The loop body contains only `{>, <, +, -}`
3. **Fixed per-iteration effect**: Each iteration applies a constant additive change to each visited cell
4. **Control cell decrement**: The current cell (at offset 0) is decremented by a constant d > 0 per iteration

### Termination Safety

For the optimization to be **safe**, we must guarantee that the loop terminates (i.e., the control cell eventually reaches zero).

Let **d** be the per-iteration decrement of the control cell.

#### Mathematical Analysis

Under modulo 256 arithmetic (ℤ/256ℤ), consider the sequence generated by repeatedly subtracting d from an initial value x:

```
x, x-d, x-2d, x-3d, ..., x-kd, ...  (mod 256)
```

This sequence reaches 0 if and only if there exists some k such that:

```
x - kd ≡ 0 (mod 256)
```

This is equivalent to:

```
kd ≡ x (mod 256)
```

**By Bézout's identity**, this equation has a solution for k if and only if **gcd(d, 256) divides x**.

#### Termination Guarantee

Since we do not know the initial value x at compile time, we can only guarantee termination for all possible initial values if:

```
gcd(d, 256) = 1
```

This means **d must be coprime to 256**.

Since 256 = 2^8, a number is coprime to 256 if and only if it is **odd**.

#### The Odd Requirement

**Therefore, termination is guaranteed for all initial values if and only if d is odd.**

If d is even:
- gcd(d, 256) ≥ 2
- The loop only visits values congruent to x modulo gcd(d, 256)
- If x is not divisible by gcd(d, 256), the loop **never terminates**

**Example of non-termination:**
```bf
+[--]   # Set cell to 1, then try to decrement by 2
```

Sequence: 1 → 255 → 253 → 251 → ... → 3 → 1 → 255 → ...
The loop cycles through odd numbers forever.

#### Compiler Policy

**CBT-FUCK only applies the multiplication loop optimization when the decrement factor d is odd.**

This conservative approach guarantees correctness for all possible runtime values.

### Examples

**Valid multiplication loops** (can be safely optimized):

```bf
[->+<]                  # d=1: Copy cell 0 to cell 1
[->>++<<]               # d=1: Add 2*(cell 0) to cell 2
[-<+>>+++<]             # d=1: Add cell 0 to cell -1, add 3*(cell 0) to cell 2
[--->>++>>>+++++<<<<<]  # d=3: Complex transfer with odd decrement
```

**Invalid loops** (rejected by optimizer):

```bf
[-->>+<<]    # d=2 (even): Unsafe, infinite loop if cell 0 is odd
[---->>+<<]  # d=4 (even): Unsafe, infinite loop unless cell 0 divisible by 4
```

### Mathematical Foundation: The d=1 Case

Consider the simplest and most common case where d=1 (decrement by 1 each iteration). A typical loop looks like:

```bf
[->>+++<<]  # Copy cell 0 to cell 2, scaled by 3
```

**Loop semantics:**
- Each iteration: `ptr[0] -= 1`, `ptr[2] += 3`
- Loop runs while `ptr[0] != 0`
- Starting with `ptr[0] = x`, the loop runs for exactly **x iterations**
- Final result: `ptr[0] = 0`, `ptr[2] = ptr[2] + 3x`

**General form for d=1:**
```
If ptr[0] = x initially, and each iteration adds s to ptr[m]:
  Final state: ptr[m] += s * x, ptr[0] = 0
```

This is straightforward because the number of iterations equals the initial value.

### Mathematical Foundation: The d≠1 Case

When d > 1 and odd, the analysis requires modular arithmetic. Consider:

```bf
[--->>++<<]  # d=3, adds 2 to cell 2 per iteration
```

**Loop semantics:**
- Each iteration: `ptr[0] -= 3`, `ptr[2] += 2`
- Starting with `ptr[0] = x`, we need to find how many iterations k until `ptr[0] = 0`
- This requires solving: `x - 3k ≡ 0 (mod 256)`, or equivalently `3k ≡ x (mod 256)`
- Since gcd(3, 256) = 1, the solution is: `k ≡ x · 3⁻¹ (mod 256)`
- The modular inverse 3⁻¹ ≡ 171 (mod 256), so k ≡ 171x (mod 256)

**Total effect on cell 2:**
```
ptr[2] += 2k ≡ 2 · 171x ≡ 342x ≡ 86x (mod 256)
```

**General form for odd d:**
```
If ptr[0] = x initially, decrement by d per iteration, add s to ptr[m] per iteration:
  Number of iterations: k ≡ x · d⁻¹ (mod 256)
  Final state: ptr[m] += s · k ≡ (s · d⁻¹) · x (mod 256), ptr[0] = 0
```

The key insight: the effective scaling factor is **s · d⁻¹ (mod 256)**, not just s.

### Optimized Code Generation

#### Case 1: d=1 (Most Common)

For loops with per-iteration decrement d=1:

```c
uint8_t x = ptr[0];  // Save initial control cell value

// Apply cumulative effects in O(1) time
ptr[m1] = (uint8_t)(ptr[m1] + s1 * x);
ptr[m2] = (uint8_t)(ptr[m2] + s2 * x);
// ... for each affected cell

// Control cell consumed
ptr[0] = 0;
```

Where s₁, s₂, ... are the per-iteration additive changes to each cell.

In this case, if the scaling factor for a tape cell is 1, then we avoid the multiplication operation entirely.

```c
// Instead of: ptr[1] += 1 * x; (requires multiplication)
ptr[1] += x;  // Direct addition
```

**Example:** `[->>+++<<]` compiles to:
```c
uint8_t x = ptr[0];
ptr[2] = (uint8_t)(ptr[2] + 3 * x);
ptr[0] = 0;
```

#### Case 2: d>1 and odd (Advanced)

For loops with odd per-iteration decrement d>1:

```c
uint8_t x = ptr[0];  // Save initial control cell value

// Compute modular inverse of d (can be precomputed at compile time)
uint8_t d_inv = modinv(d, 256);

// Apply cumulative effects with effective scaling factors
ptr[m1] = (uint8_t)(ptr[m1] + (uint8_t)(s1 * d_inv * x));
ptr[m2] = (uint8_t)(ptr[m2] + (uint8_t)(s2 * d_inv * x));
// ... for each affected cell

// Control cell consumed
ptr[0] = 0;
```

Where s₁, s₂, ... are the per-iteration changes, and the effective scaling factors are s₁·d⁻¹, s₂·d⁻¹, etc.

**Example:** `[--->>++<<]` where d=3, s=2, d⁻¹≡171 (mod 256):
```c
uint8_t x = ptr[0];
ptr[2] = (uint8_t)(ptr[2] + (uint8_t)(342 * x));  // 342 = 2 * 171
ptr[0] = 0;
```

Note: 342 ≡ 86 (mod 256), so this could be optimized to `ptr[2] + 86*x` at compile time.

### Correctness Proof

**Claim:** The optimized code is semantically equivalent to the original loop.

**Proof for d=1:**

1. **Termination:** Loop runs while `ptr[0] != 0`. Starting at x, after x iterations of `-=1`, we have `ptr[0] = 0`. ✓

2. **Control cell:** Final value is 0 in both cases. ✓

3. **Other cells:** Each cell receives s per iteration for x iterations, total s·x. This matches the optimized code. ✓

**Proof for odd d>1:**

1. **Termination:** Since gcd(d, 256) = 1, there exists unique k (mod 256) such that kd ≡ x (mod 256), namely k ≡ x·d⁻¹. The loop terminates after k iterations. ✓

2. **Control cell:** After k iterations of `-=d`, we have `ptr[0] ≡ x - kd ≡ 0 (mod 256)`. ✓

3. **Other cells:** Each cell receives s per iteration for k iterations. Total contribution:
   ```
   s · k ≡ s · (x·d⁻¹) ≡ (s·d⁻¹) · x (mod 256)
   ```
   This matches the optimized code. ✓

### Performance Impact

**Time complexity:**
- **Before:** O(x/d) iterations, where x ≤ 255
- **After:** O(1) constant time (just a few arithmetic operations)

**Space complexity:**
- No change (same memory usage)

**Code size:**
- Smaller (eliminates loop structure)
- Enables further optimizations by C compiler

**Maximum speedup:**
- Up to **255× faster** for large initial values when d=1
- Up to **85× faster** for d=3 (since 255/3 ≈ 85)

This is one of the most impactful optimizations in the CBT-FUCK compiler, transforming O(n) loops into O(1) arithmetic.

---