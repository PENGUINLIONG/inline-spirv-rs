//! # jit-spirv
//!
//! The first parameter is a slice of string that contains the textual shader
//! source. Other following parameters give you finer control over the generated
//! code to compile your shaders.
//!
//! ## Source Language
//!
//! `jit-spirv` currently support three source languages:
//!
//! - `glsl`: The shader source is in GLSL (enabled by default);
//! - `hlsl`: The shader source is in HLSL (enabled by default);
//! - `wgsl`: The shader source is in WGSL.
//!
//! The experimental WGSL support for WebGPU is available when `wgsl` feature is
//! enabled, but currently you have to compile with a nightly toolchain. Limited
//! by the `naga` backend, most of the extra parameters won't be effective and
//! only the first entry point is generated in SPIR-V.
//!
//! ## Shader Stages
//!
//! The following shader stages are supported:
//!
//! - `vert`: Vertex shader;
//! - `tesc`: Tessellation control shader (Hull shader);
//! - `tese`: Tessellation evaluation shader (Domain shader);
//! - `geom`: Geometry shader;
//! - `frag`: Fragment shader (Pixel shader);
//! - `comp`: Compute shader;
//! - `mesh`: (Mesh shading) Mesh shader;
//! - `task`: (Mesh shading) Task shader;
//! - `rgen`: (Raytracing) ray-generation shader;
//! - `rint`: (Raytracing) intersection shader;
//! - `rahit`: (Raytracing) any-hit shader;
//! - `rchit`: (Raytracing) closest-hit shader;
//! - `rmiss`: (Raytracing) miss shader;
//! - `rcall`: (Raytracing) callable shader;
//!
//! ## Specify Entry Function
//!
//! By default the compiler seeks for an entry point function named `main`. You
//! can also explicitly specify the entry function name:
//!
//! ```ignore
//! jit_spirv!(hlsl_source, hlsl, vert, entry="very_main");
//! ```
//!
//! ## Optimization Preference
//!
//! To decide how much you want the SPIR-V to be optimized:
//!
//! - `min_size`: Optimize for the minimal output size;
//! - `max_perf`: Optimize for the best performance;
//! - `no_debug`: Strip off all the debug information (don't do this if you want
//! to reflect the SPIR-V and get variable names).
//!
//! ## Compiler Definition
//!
//! You can also define macro substitutions:
//!
//! ```ignore
//! jit_spirv!(glsl_source, vert,
//!     D USE_LIGHTMAP,
//!     D LIGHTMAP_COUNT="2");
//! ```
//!
//! You can request a specific version of target environment:
//! - `vulkan1_0` for Vulkan 1.0 (default, supports SPIR-V 1.0);
//! - `vulkan1_1` for Vulkan 1.1 (supports SPIR-V 1.3);
//! - `vulkan1_2` for Vulkan 1.2 (supports SPIR-V 1.5).
//! - `opengl4_5` for OpenGL 4.5 core profile.
//! - `webgpu` for WebGPU.
//!
//! Of course once you started to use macro is basically means that you are
//! getting so dynamic that this little crate might not be enough. Then it might
//! be a good time to build your own shader compilation pipeline!
//!
//! ## Descriptor Auto-binding
//!
//! If you are just off your work being tooooo tired to specify the descriptor
//! binding points yourself, you can switch on `auto_bind`:
//!
//! ```ignore
//! jit_spirv!(r#"
//!     #version 450 core
//!     uniform sampler2D limap;
//!     uniform sampler2D emit_map;
//!     void main() {}
//! "#, glsl, frag, auto_bind);
//! ```
//!
//! However, if you don't have any automated reflection tool to get the actual
//! binding points, it's not recommended to use this.
//!
//! ## Flip-Y for WebGPU
//!
//! If you intend to compile WGSL for a WebGPU backend, `naga` by default
//! inverts the Y-axis due to the discrepancy in NDC (Normalized Device
//! Coordinates) between WebGPU and Vulkan. If such correction is undesired, you
//! can opt out with `no_y_flip`.
pub mod dep;
pub use jit_spirv_impl::jit_spirv;

pub struct CompilationFeedback {
    pub spv: Vec<u32>,
    pub dep_paths: Vec<String>,
}
