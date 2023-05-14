use jit_spirv::jit_spirv;
use spirq::ReflectConfig;

fn main() {
    let glsl_source =
        r#"
        #version 450 core
        layout(constant_id = HACK_SCALE_CONSTANT_ID) const float hack_scale = 0;

        layout(location = 1)
        in vec2 uv;
        layout(location = 0)
        out vec4 color;

        uniform sampler2D limap;
        uniform sampler2D emit_map;

        void main() {
            color = texture(limap, uv) + texture(emit_map, uv) * hack_scale;
        }
    "#;

    let frag = jit_spirv!(
        glsl_source,
        frag,
        auto_bind,
        D HACK_SCALE_CONSTANT_ID="233",
    ).unwrap();

    let entry_points = ReflectConfig::new().spv(frag.spv).reflect().unwrap();
    let entry = entry_points.first().unwrap();

    println!("{:#?}", entry);
}