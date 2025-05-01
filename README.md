# Ratio
Ratio is a fast, natively compiled, statically typed work-in-progress programming language. This repository contains the compiler for Ratio. The syntax is similar to Python, Rust, and TypeScript. Below is a simple example program.

```
fn main():
    print("Hello world")
    var x: int = 10
    print(x)
```

## Building and running
Simply run `cargo run` to build Ratio and compile the code in `input.ratio`! It will create an executable file `output`. Note that you will need GCC (we use it to link) and NASM for this to work, as these are called internally by Ratio.

## A note on the name
Ratio is probably going to be a temporary name, just because I couldn't think of anything else.
