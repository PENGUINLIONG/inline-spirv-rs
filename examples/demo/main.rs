use inline_spirv::{inline_spirv, include_spirv as include_spirv_raw};
use spirq::{SpirvBinary};
use env_logger;
use log::info;

// Notice how you can make a more customized version of include macro, same for
// the inline macro.
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
    env_logger::init();
    let vert: &[u32] = include_spirv!("examples/demo/assets/demo.hlsl", vert);
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

    info!("vertex shader:\n{:#?}", vert.iter().copied().collect::<SpirvBinary>().reflect().unwrap()[0]);
    info!("fragment shader:\n{:#?}", frag.iter().copied().collect::<SpirvBinary>().reflect().unwrap()[0]);

    info!("sounds good");
}
