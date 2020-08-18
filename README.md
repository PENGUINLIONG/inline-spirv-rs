# Inline SPIR-V

[![Crate](https://img.shields.io/crates/v/inline-spirv)](https://crates.io/crates/inline-spirv)
[![Documentation](https://docs.rs/inline-spirv/badge.svg)](https://docs.rs/inline-spirv)

`inline-spirv` ease the way you write shaders. Although as game developers, we usually compile a permutation of shader stages for different objects and materials at runtime for the best flexibility; sometimes we *do* want to try out some new ideas and start up dirty. This crate helps you compile GLSL/HLSL shader in your Rust code, or in external files, into SPIR-V; and embed them right inside the binary, so you are freed from all the compilation hassle.

## How to Use

To inline shader source:

```rust
use inline_spirv::include_spirv;

let spv: &'static [u32] = inline_spirv!(r#"
    #version 450 core
    void main() { gl_Position = vec4(0, 0, 0, 1); }
"#, vert);
```

To include a external shader source file:

```rust
use inline_spirv::include_spirv;

let spv: &'static [u32] = include_spirv!("assets/vert.hlsl", vert, hlsl, entry="Main");
```

For the full list of options please refer to the [documentation](https://docs.rs/inline-spirv).

## Tips

The macro can be verbose especially you have a bunch of `#include`s, so please be aware of that you can alias and define a more customized macro for yourself:

```rust
use inline_spirv::include_spirv as include_spirv_raw;

macro_rules! include_spirv {
    ($path:expr, $stage:ident) => {
        include_spirv_raw!(
            $path,
            $stage, hlsl,
            entry="my_entry_pt",
            D VERBOSE_DEFINITION,
            D ANOTHER_VERBOSE_DEFINITION="verbose definition substitution",
            I "long/path/to/include/directory",
        )
    }
}

// ...
let vert: &[u32] = include_spirv!("examples/demo/assets/demo.hlsl", vert);
```

## License

This project is licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
