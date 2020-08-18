#include <assets/counter.hlsl>
#include "counter.hlsl"

[[vk::push_constant]]
cbuffer pcBuf {
    float4x4 view;
};

[[vk::binding(COUNTER, DESC_SET)]]
Texture2D tex;
[[vk::binding(COUNTER, DESC_SET)]]
SamplerState nearest2D;

struct Attribute {
    [[vk::location(0)]]
    float3 world_pos;
    [[vk::location(1)]]
    float3 norm;
    [[vk::location(2)]]
    float2 uv;
#ifdef USE_COLOR
    [[vk::location(3)]]
    float4 color;
#endif // USE_COLOR
};
struct Varying {
    float4 pos: SV_POSITION;
#ifdef USE_COLOR
    float4 color: COLOR;
#endif // USE_COLOR
};

Varying vertex_shader(Attribute attr) {
    Varying var;
    var.pos = float4(attr.world_pos + attr.norm * tex.Sample(nearest2D, attr.uv), 1) * view;
#ifdef USE_COLOR
    var.color = attr.color;
#endif // USE_COLOR
    return var;
}
