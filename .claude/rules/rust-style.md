---
description: Rust programming style guidelines emphasizing simplicity
globs:
alwaysApply: true
---

# Rust Programming Style Rules

## Reference

Our Rust programming style is based on TigerBeetle's "Tiger Style." For comprehensive guidelines, refer to: [Tiger Style](https://github.com/tigerbeetle/tigerbeetle/blob/0.16.54/docs/TIGER_STYLE.md)

## Design Goals

Our primary objectives are:

1. **Safety**: Ensure code correctness and prevent undefined behavior
2. **Performance**: Write efficient code without unnecessary overhead
3. **Developer Experience**: Maintain readability and ease of maintenance

Simplicity and elegance are key to achieving these goals. As Edsger Dijkstra noted, "Simplicity and elegance are unpopular because they require hard work and discipline to achieve."

## Core Principles

### Simplicity Over Complexity
- Implement correctly the first time
- Avoid shortcuts that create maintenance burden

### Prefer Functional Over OOP
- Use functions and structs over complex inheritance hierarchies
- Structure code around data transformations
- Favor composition over inheritance
- Prefer stateless functions when possible (state machines and similar patterns are exceptions)
- Prefer functions that return concrete types over functions that return `()`

## Code Structure

### Avoid Over-Engineering
- Minimize unnecessary abstractions and trait hierarchies
- Use simple control flow, if/else and exhaustive pattern matching
- Use enums and strong typing for exhaustive pattern matching
- No recursion - use iteration (Rust can optimize recursion with TCO, but prefer iteration for clarity)
- Set explicit upper bounds on loops

### Performance and Safety
- Consider performance and safety from the start
- Use assertions to detect errors early, see assertion style rules
- Handle errors explicitly with `Result<T, E>` and `Option<T>`
- Always consider our resources: bandwidth and latency
- Set resource limits: network, disk, memory, CPU

### Memory and Resource Efficiency
- Prefer stack allocation for small, fixed-size data
- Use heap allocation only for large or dynamically sized data
- Minimize dynamic memory allocation in performance-critical code
- Use `Vec::with_capacity()` when size is known
- Prefer `&[T]` over `&Vec<T>` in function signatures
- Use `std::mem::take()` for efficient moves
- Leverage Rust's ownership model for automatic resource management
- Use `const` functions for compile-time evaluation
- Prefer iterators over manual indexing for data processing

### TigerBeetle-Inspired Patterns
- Allocate all memory at startup when possible (static allocation strategy)
- Use zero-copy operations and direct I/O where applicable
- Align data structures to cache line boundaries for performance
- Implement deterministic simulation testing for complex systems
- Use comprehensive assertions to validate all function boundaries
- Follow NASA's Power of Ten Rules for safety-critical code
- Minimize data copying and deserialization overhead
- Use fixed-size buffers and pre-allocated memory pools

#### Memory Efficiency Examples
```rust
// Good: Stack allocation for small data
fn process_small_data() {
    let buffer: [u8; 1024] = [0; 1024];
    // Use buffer...
}

// Good: Pre-allocated capacity
fn process_known_size() {
    let mut data = Vec::with_capacity(1000);
    // Add items without reallocations
}

// Good: Borrowing over cloning
fn process_data(data: &[u8]) -> usize {
    data.len() // No allocation
}

// Good: Efficient moves
fn take_ownership(mut data: Vec<u8>) -> Vec<u8> {
    let result = std::mem::take(&mut data);
    // data is now empty, result has the data
    result
}

// Good: Compile-time evaluation
const fn calculate_size() -> usize {
    1024 * 1024 // Computed at compile time
}

// TigerBeetle style: Static allocation at startup
struct System {
    buffer_pool: [Buffer; 1000], // Pre-allocated at startup
    free_buffers: Vec<usize>,    // Indices of free buffers
}

// TigerBeetle style: Cache-aligned data structures
#[repr(align(64))] // Cache line alignment
struct CacheAlignedData {
    data: [u8; 64],
}

// TigerBeetle style: Zero-copy operations
fn process_data_zero_copy(data: &[u8]) -> &[u8] {
    // No allocation, just return a slice
    &data[10..20]
}
```

## Naming and Documentation

### Documentation
- Explain why, not what
- Document decisions
- Keep comments current
- Write minimal but meaningful documentation
- Do not use emojis
- Do not add unnecessary comments
- Assume high seniority of software engineers will read the code

## Code Organization

### File Structure
- Single responsibility per file
- Group related functionality
- Minimize dependencies
- Define clear interfaces

### Function Design
- Keep functions small and focused
- Prefer pure functions when possible (state machines and similar patterns are exceptions)
- Use explicit parameters
- Prefer return values over side effects

## Anti-Patterns

- Premature abstractions and too many files
- Complex trait hierarchies
- Deep inheritance (use composition instead)
- Over-engineering with unnecessary generics or macros
- Recursion when performance matters and compiler can't optimize
- Complex conditionals
- Complex nested loops
- Panic-driven flow (use `Result` and `Option`)
- Global state
- Hidden state
- Complex state machines
- Unnecessary heap allocations in hot paths
- Using `clone()` when borrowing would suffice
- Ignoring memory layout and cache locality
- Using `Vec::new()` when capacity is known
- Using `String::new()` when size is known
- Not using `const` for compile-time computations
- Using `unsafe` when safe alternatives exist
- Dynamic allocation during runtime when static allocation is possible
- Ignoring cache line alignment for performance-critical data
- Unnecessary data copying and deserialization
- Missing assertions at function boundaries
- Not following NASA's Power of Ten Rules for safety-critical code

## Anti-Clean Code Practices

- Don't follow clean code practices
- Too many small functions
- Inline simple functions instead of extracting them
- Prefer longer functions over many small ones
- Don't extract every single line into a function
- Avoid function extraction for trivial operations
- Keep related logic together in one function
