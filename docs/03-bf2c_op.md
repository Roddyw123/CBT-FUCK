# Brainfuck to C Compilation: Optimization

The translation from Brainfuck to C is considered trivial due to the semantic closeness of these languages. Therefore, the primary focus of CBT-FUCK's compilation pipeline is the generation of highly optimized C code through pattern recognition and algebraic transformation.

This document describes the optimization passes applied during compilation, their mathematical foundations, and the correctness guarantees they provide.

Here is a table of contents for all of the current optimizations:

1. [Instruction Coalescing and Cancellation](#instruction-coalescing-and-cancellation)
2. [Zero Loops](#zero-loops)
3. [Scan Loops](#scan-loops)
4. [Multiplication Loops](#multiplication-loops)
5. [Offset Optimization](#offset-optimization)
6. [Conditional Conversion](#conditional-conversion)
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

## Offset Optimization
It is recommended that you read how instruction coalescing and cancellation work before reading this section.

**Offset optimization** eliminates redundant pointer movements by using direct offset addressing instead of physically moving the pointer and then moving it back.

This optimization operates on the coalesced IR (after instruction coalescing has already been applied) and transforms sequences where the pointer moves away from its starting position, performs operations, then returns.

### Problem Statement

Consider the pattern:
```bf
>+++<
```

After instruction coalescing, this becomes:
```
Move(1), Add(3), Move(-1)
```

The naive compilation generates:
```c
ptr++;
*ptr += 3;
ptr--;
```

The pointer movements are **redundant**, the pointer ends where it started, so we could have used offset addressing instead.

### Key Insight

In C, array indexing `ptr[n]` is equivalent to dereferencing `*(ptr + n)`. This allows us to access cells at offset positions without physically moving the pointer variable.

Instead of:
```c
ptr++;        // Move to offset 1
*ptr += 3;    // Modify
ptr--;        // Move back
```

We can write:
```c
ptr[1] += 3;  // Access offset 1 directly
```

We may also use offset optimization on I/O operations:

### Recognition Conditions

A sequence of operations qualifies for offset optimization when:

1. **Net pointer movement is non-zero** over a subsequence of instructions
2. **Operations occur at various offsets** from the starting position
3. **No intervening loops or I/O** that require the physical pointer to be at a specific position

The optimizer tracks a **virtual pointer offset** during code generation, delaying physical pointer movements until necessary (loop entries, I/O operations, or end of basic blocks).

### Examples

**Example 1: Zero net movement**

```bf
>++>+++<<<<  # Net movement: 0
```

Generates:
```c
ptr[1] += 2;
ptr[2] += 3;
// No pointer movement needed
```

**Example 2: Non-zero net movement**

```bf
>++>+++  # Net movement: +2
```

Generates:
```c
ptr[1] += 2;
ptr[2] += 3;
ptr += 2;  // Physical pointer catches up
```

**Example 3: Flush before loop**

```bf
>++[...]  # Operations before loop
```

Generates:
```c
ptr[1] += 2;
ptr++;      // Must flush before loop
while (*ptr) { ... }
```

The physical pointer must be correct before loops because the loop condition tests `*ptr`, not `ptr[offset]`.

**Example 4: Flush before I/O**

```bf
>+++.  # Move right, add 3, then output
```

Generates:
```c
ptr[1] += 3;
ptr++;        // Must flush before I/O
putchar(*ptr);
```

I/O operations require the physical pointer to be at the correct position because they read/write `*ptr` at the current location.

### Correctness

**Claim:** Offset optimization preserves program semantics.

**Proof:**

1. **Cell values:** Array indexing `ptr[n]` is semantically equivalent to `*(ptr + n)`, so `ptr[n] = v` is equivalent to moving the pointer by `n`, assigning `*ptr = v`, then moving back by `-n`. ✓

2. **Pointer position:** Physical pointer position is synchronized at all observable points:
   - Loop conditions (test `*ptr`)
   - I/O operations (read/write `*ptr`)
   - Program termination

   Between these points, the physical pointer may differ from the logical position, but this discrepancy is not observable. ✓

3. **Operation ordering:** Operations on the same cell are applied in program order. Operations on different cells are independent and can be reordered. ✓

### Performance Impact

**Improved characteristics:**
- Less redundant pointer arithmetic
- Better cache behavior (less pointer mutation)
- Improved instruction-level parallelism (independent offset accesses)

For `>+>++>+++<<<` repeated 1000 times:
- **Without optimization:** 6,000 pointer operations + 3,000 arithmetic
- **With optimization:** 0 pointer operations + 3,000 arithmetic
- **Speedup:** ~1.5-2× depending on architecture

---

## Conditional Conversion
Not all loops in Brainfuck are loops in C. If we know that a loop in Brainfuck executes at most once, then it may be an if-statement in the C Programming Language.

**Conditional Conversion** involves converting Brainfuck loops are known to execute at most once, and then converting them to conditionals in the C code rather than a standard, unoptimized while loop. This is done to reduce loop overhead.

### Conversion Condition
A Brainfuck loop may be converted to a if-statement rather than a while loop if and only if:
1. There is zero net pointer movement, the loop must return the pointer to its starting position.
2. The loop must not contain other Loop statements (general loops), MultiplicationLoop statements (these are already optimized), ScanLoop statements (these modify pointer position unpredictably)

### Conditional Cases
There are two cases of conditional conversion, one which is trivial and the other which is very complex to implement.

#### Case 1: Zero Loop Conditional: Trivial

If a Brainfuck Loop satisfies all the Conversion Conditions and contains a `ZeroLoop` statement at offset 0 (the current cell position when tracking cumulative pointer movement through the loop body), then the loop is guaranteed to execute at most once.

**Detection:**
As we trace through the loop body tracking cumulative pointer offset, if we encounter a `ZeroLoop` when the current offset equals 0, the loop unconditionally zeros the current cell and will terminate after at most one iteration. This guarantees single iteration because:
- If `*ptr = 0` initially: loop condition is false, body never executes
- If `*ptr ≠ 0` initially: loop executes, `ZeroLoop` sets `*ptr = 0`, loop condition becomes false, terminates after exactly 1 iteration

**Examples:**

**Example 1:** Zero with side effects after
```bf
[[-]>+++<]
```

**IR form:**
```
Loop([
    ZeroLoop,      // At offset 0
    Move(1),       // Offset becomes 1
    Add(3),        // Modify offset 1
    Move(-1)       // Return to offset 0
])
```

**Converted to:**
```c
if (*ptr != 0) {
    *ptr = 0;
    ptr[1] += 3;
}
```

**Example 2:** Zero with side effects before
```bf
[>+++<[-]]
```

**IR form:**
```
Loop([
    Move(1),       // Offset becomes 1
    Add(3),        // Modify offset 1
    Move(-1),      // Return to offset 0
    ZeroLoop       // At offset 0
])
```

**Converted to:**
```c
if (*ptr != 0) {
    ptr[1] += 3;
    *ptr = 0;
}
```

**Example 3:** Degenerate case when the body only contains a zeroloop.
```bf
[[-]]
```

**IR form:**
```
Loop([ZeroLoop])
```

This is a special optimization! Since the body only contains `ZeroLoop` and nothing else, we can skip the conditional entirely:
```c
*ptr = 0;  // Setting 0→0 is a no-op, so no if-check needed
```

#### Case 2: Boolean Value Conditional: Non-trivial
Suppose we have a loop that follows all conversion conditions but doesn't adhere to case 1. If we know the cell can only be 0 or 1 (a boolean), then the loop executes **at most once**.

**Conditions:**

For this optimization to apply:
1. The loop must satisfy all Conversion Conditions (zero net pointer movement, no nested loops)
2. The loop body must change the current cell by exactly ±1 per iteration (unit increment or decrement)
3. We must **prove** that the current cell can only hold values 0 or 1

There are two sub-cases for this case:

**Case A: Cell is 0**
```
Initial: *ptr = 0
Loop condition (*ptr != 0): FALSE
Result: Loop body never executes
```

**Case B: Cell is 1**
```
Initial: *ptr = 1
Loop condition (*ptr != 0): TRUE → execute body
Body decrements by 1: *ptr becomes 0
Loop condition (*ptr != 0): FALSE
Result: Loop body executes exactly once, then terminates
```

Since the value is provably 0 or 1, **no other cases exist**. The loop cannot execute more than once.

In order to prove that a cell is boolean, ae need to track what values each cell might contain as we process the program. This is called **value range analysis**.

Think of it like this: each cell has a "possible values" range:
- After `[-]`: range is `[0, 0]` (definitely 0)
- After `[-]+`: range is `[1, 1]` (definitely 1)
- After `[-]++`: range is `[2, 2]` (definitely 2)
- After unknown operations: range is `[0, 255]` (could be anything)

A cell is **provably boolean** when its range is `[0, 1]` (or narrower like `[0, 0]` or `[1, 1]`).

---

**Example 1: Simple boolean flag**

```bf
[-]    # Zero the cell: range becomes [0, 0]
+      # Increment: range becomes [1, 1]
[>+++<-]
```

**Step-by-step analysis:**
1. After `[-]`: Cell 0 has range `[0, 0]`
2. After `+`: Cell 0 has range `[1, 1]` ✓ This is boolean!
3. Loop `[>+++<-]` body analysis:
   - Decrements cell 0 by 1 per iteration ✓
   - Returns to starting position ✓
   - No nested loops ✓

**Since cell 0 is provably `1`, we know the loop executes exactly once:**

```c
*ptr = 0;
*ptr = 1;
// Instead of: while (*ptr) { ptr[1] += 3; (*ptr)--; }
if (*ptr != 0) {  // We know this is always true
    ptr[1] += 3;
    (*ptr)--;
}
```

Since we know `*ptr = 1` at this point, we can use constant propagation to simplify:
```c
ptr[1] += 3;
*ptr = 0;
```

---

**Example 2: Boolean from conditional operation**

```bf
[-]>      # Zero cell 0, move to cell 1
[<+>-]    # Transfer cell 1 to cell 0 (if cell 1 was non-zero)
<         # Return to cell 0
[>+<-]    # Our target loop
```

**Analysis:**
- The transfer operation `[<+>-]` is a common pattern that creates a boolean:
  - If cell 1 was `0`: cell 0 becomes `0`
  - If cell 1 was `1`: cell 0 becomes `1`
  - If cell 1 was `5`: cell 0 becomes `5` (but this makes cell 0 NOT boolean!)

**Without tracking cell 1's range,** we cannot prove cell 0 is boolean after the transfer.

**But if we know cell 1 is boolean** (from earlier analysis), then cell 0 becomes boolean too!

This shows why value range analysis must track **all cells**, not just the current one.

---

**Example 3: Normalize any value to boolean**

```bf
[[-]+]     # If cell non-zero: zero it, then set to 1
```

**Execution trace:**
- If `*ptr = 0`: Loop doesn't execute, `*ptr` stays `0`
- If `*ptr = 5`: Loop executes once, `ZeroLoop` sets to `0`, then `+` sets to `1`
- Result: `*ptr ∈ [0, 1]` ✓ Boolean!

---

**When does Case 2 apply instead of MultiplicationLoop?**

Consider this pattern:
```bf
[>+++<-]
```

**Without value range analysis:**
- We don't know if the cell is boolean
- Optimized as `MultiplicationLoop`: `ptr[1] += 3 * (*ptr); *ptr = 0;`

**With value range analysis proving boolean:**
- We know `*ptr ∈ [0, 1]`
- Can optimize as conditional: `if (*ptr) { ptr[1] += 3; *ptr = 0; }`

**Why prefer the conditional?**
```c
// MultiplicationLoop (works for any value)
uint8_t x = *ptr;
ptr[1] += 3 * x;  // Requires multiplication
*ptr = 0;

// Conditional (when boolean)
if (*ptr != 0) {  // Only checks 0 or 1
    ptr[1] += 3;  // Just addition, no multiplication!
    *ptr = 0;
}
```

The conditional version:
- Avoids multiplication (slightly faster)
- Makes the boolean nature explicit
- May enable further optimizations

---

### Invalid Examples (Cannot Convert)

**Example 1:** No zero operation
```bf
[>+<]
```
❌ Loop adds to offset 1 but never zeros the current cell. May execute multiple times depending on initial value.

**Example 2:** Zero at wrong offset
```bf
[>[-]<]
```
❌ The `ZeroLoop` occurs at offset 1, not offset 0. The current cell is not modified, so the loop condition never changes. This is an infinite loop (unless current cell is already 0).

**Example 3:** Nested control flow
```bf
[[-]>>[+]<<]
```
❌ Contains multiple `ZeroLoop` statements and complex control flow. Rejected due to Conversion Condition 2 (no nested loops).

**Example 4:** Non-zero net pointer movement
```bf
[[-]>]
```
❌ Net pointer movement is +1, violates Conversion Condition 1.

**Example 5:** Multiplication loop pattern
```bf
[>+++<-]
```
❌ This is a unit-decrement loop, but **without value range analysis** we cannot prove the cell is boolean. This pattern is already optimized by the MultiplicationLoop optimization:
```c
uint8_t x = *ptr;
ptr[1] += 3 * x;
*ptr = 0;
```

However, **with value range analysis**, if we can prove `*ptr ∈ [0, 1]`, then this qualifies for Case 2 conversion.

### Correctness Proof

**Claim:** Converting qualifying loops to if-statements preserves program semantics.

**Proof for Case 1 (ZeroLoop present):**

Let L = `Loop(body)` where body contains `ZeroLoop` at offset 0.

**When `*ptr = 0` initially:**
- Original: Loop condition `*ptr != 0` is false. Body does not execute.
- Converted: If condition `*ptr != 0` is false. Body does not execute.
- Final state identical. ✓

**When `*ptr ≠ 0` initially (let `*ptr = n` where n ∈ [1, 255]):**
- Original: Loop executes body. Body contains `ZeroLoop` which sets `*ptr = 0`. Loop condition becomes false. Loop terminates after exactly 1 iteration.
- Converted: If condition `*ptr != 0` is true. Body executes once. Body contains `ZeroLoop` which sets `*ptr = 0`.
- Both execute body exactly once with identical side effects. ✓

**Pointer position:**
Both forms maintain pointer position since Conversion Condition 1 requires zero net movement. ✓

**Side effects:**
All operations in body execute in the same order in both forms. I/O operations, memory modifications, and pointer movements are identical. ✓

**Proof for Case 2 (Boolean + unit decrement):**

Let L = `Loop(body)` where:
- body decrements offset 0 by 1 per iteration
- value range analysis proves `*ptr ∈ [0, 1]`

**When `*ptr = 0` initially:**
- Original: Loop condition false. Body does not execute.
- Converted: If condition false. Body does not execute.
- Final state identical. ✓

**When `*ptr = 1` initially:**
- Original: Loop executes body. Body decrements `*ptr` by 1, so `*ptr` becomes 0. Loop condition becomes false. Loop terminates after exactly 1 iteration.
- Converted: If condition true. Body executes once, decrements `*ptr` to 0.
- Both execute body exactly once with identical side effects. ✓

**Impossibility of other values:**
Value range analysis guarantees `*ptr ∈ [0, 1]`, so no other cases exist. ✓

### Performance Impact
1. **Eliminates loop back-edge:** No jump instruction back to loop start
2. **Eliminates redundant condition check:** Loop checks condition twice (entry + potential re-entry), if-statement checks once
3. **Better branch prediction:** Modern CPUs handle if-statements slightly better than small loops
4. **Improved code generation:** Compilers generate simpler code for if-statements
- Modest **5-10%** speedupfor code with this pattern
- Primarily benefits code clarity rather than raw performance
- The real benefit is **semantic clarity** by making single-execution intent explicit to both humans and optimizers
