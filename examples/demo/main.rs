use inline_spirv::{inline_spirv, include_spirv};
use spirq::{SpirvBinary};
use env_logger;
use log::info;

fn main() {
    env_logger::init();
    let vert: &[u32] = include_spirv!(
        "examples/demo/assets/demo.hlsl",
        vert, hlsl,
        entry="vertex_shader",
        D USE_COLOR,
        D DESC_SET="7",
        I "examples/demo",
    );
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
