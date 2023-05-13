# JIT SPIR-V

[![Crate](https://img.shields.io/crates/v/jit-spirv)](https://crates.io/crates/jit-spirv)
[![Documentation](https://docs.rs/jit-spirv/badge.svg)](https://docs.rs/jit-spirv)

`jit-spirv` helps you integrate SPIR-V shader compilers into your project with minimal amount of code.

## How to Use

To compile a runtime shader source just-in-time:

```rust
use jit_spirv::{jit_spirv, CompilationFeedback};

let glsl_source = r#"
    #version 450
    layout(binding=0) writeonly buffer _0 { float data[]; };
    void main() {
        data[gl_GlobalInvocationID.x] = 1.0;
    }
"#;
let feedback: CompilationFeedback = jit_spirv!(glsl_source, comp).unwrap();
let spv: &[u32] = &feedback.spv;
```

For the full list of options please refer to the [documentation of inline-spirv](https://docs.rs/inline-spirv).

## License

This project is licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
