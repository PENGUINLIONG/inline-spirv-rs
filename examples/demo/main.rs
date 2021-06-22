use inline_spirv::{inline_spirv, include_spirv as include_spirv_raw};
use spirq::{SpirvBinary};

// Notice how you can make a more customized version of include macro, same for
// the inline macro.
#[cfg(feature = "shaderc")]
macro_rules! include_spirv {
    ($path:expr, $stage:ident) => {
        include_spirv_raw!(
            $path,
            $stage, hlsl,
            entry="vertex_shader",
            D USE_COLOR,
            D DESC_SET="7",
            I "examples/demo",
        )
    }
}

fn main() {
    #[cfg(feature = "shaderc")]
    let vert: &[u32] = include_spirv!("examples/demo/assets/demo.hlsl", vert);

    #[cfg(feature = "shaderc")]
    let frag: &[u32] = inline_spirv!(r#"
        #version 450 core
        layout(constant_id = 233) const float hack_scale = 0;

        layout(location = 1)
        in vec2 uv;
        layout(location = 0)
        out vec4 color;

        uniform sampler2D limap;
        uniform sampler2D emit_map;

        void main() {
            color = texture(limap, uv) + texture(emit_map, uv) * hack_scale;
        }
    "#, frag, auto_bind);

    #[cfg(feature = "naga")]
    let wgsl_shader: &[u32] = include_spirv_raw!("examples/demo/assets/shader.wgsl", wgsl);

    #[cfg(feature = "naga")]
    let hello_triangle: &[u32] = inline_spirv!(r#"
        [[stage(vertex)]]
        fn vs_main([[builtin(vertex_index)]] in_vertex_index: u32) -> [[builtin(position)]] vec4<f32> {
            let x = f32(i32(in_vertex_index) - 1);
            let y = f32(i32(in_vertex_index & 1u) * 2 - 1);
            return vec4<f32>(x, y, 0.0, 1.0);
        }

        [[stage(fragment)]]
        fn fs_main() -> [[location(0)]] vec4<f32> {
            return vec4<f32>(1.0, 0.0, 0.0, 1.0);
        }
    "#, wgsl);

    #[cfg(feature = "shaderc")]
    println!("vertex shader:\n{:#?}", vert.iter().copied().collect::<SpirvBinary>().reflect_vec().unwrap()[0]);
    #[cfg(feature = "shaderc")]
    println!("fragment shader:\n{:#?}", frag.iter().copied().collect::<SpirvBinary>().reflect_vec().unwrap()[0]);

    #[cfg(feature = "naga")]
    println!("wgsl shader:\n{:#?}", wgsl_shader.iter().copied().collect::<SpirvBinary>().reflect_vec().unwrap()[0]);
    #[cfg(feature = "naga")]
    println!("hello shader:\n{:#?}", hello_triangle.iter().copied().collect::<SpirvBinary>().reflect_vec().unwrap()[0]);

    println!("sounds good");
}
