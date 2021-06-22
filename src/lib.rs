//! # inline-spirv
//!
//! The first string is always your shader path or the source code, depending on
//! the macro you use. Other following parameters give you finer control over
//! the compilation process.
//!
//! ## Source Language
//!
//! `inline-spirv` currently support three source languages:
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
//! include_spirv!("path/to/shader.hlsl", hlsl, vert, entry="very_main");
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
//! ## Include External Source
//!
//! You can use `#include "x.h"` to include a file relative to the shader source
//! file (you cannot use this in inline source); or you can use `#include <x.h>`
//! to include a file relative to any of your provided include directories
//! (searched in order). To specify a include directory:
//!
//! ```ignore
//! include_spirv!("path/to/shader.glsl", vert,
//!     I "path/to/shader-headers/",
//!     I "path/to/also-shader-headers/");
//! ```
//!
//! ## Compiler Definition
//!
//! You can also define macro substitutions:
//!
//! ```ignore
//! include_spirv!("path/to/shader.glsl", vert,
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
//! inline_spirv!(r#"
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
//!
//! ## Tips
//!
//! The macro can be verbose especially you have a bunch of `#include`s, so
//! please be aware of that you can alias and define a more customized macro for
//! yourself:
//!
//! ```ignore
//! use inline_spirv::include_spirv as include_spirv_raw;
//!
//! macro_rules! include_spirv {
//!     ($path:expr, $stage:ident) => {
//!         include_spirv_raw!(
//!             $path,
//!             $stage, hlsl,
//!             entry="my_entry_pt",
//!             D VERBOSE_DEFINITION,
//!             D ANOTHER_VERBOSE_DEFINITION="verbose definition substitution",
//!             I "long/path/to/include/directory",
//!         )
//!     }
//! }
//!
//! // ...
//! let vert: &[u32] = include_spirv!("examples/demo/assets/demo.hlsl", vert);
//! ```
extern crate proc_macro;
use std::path::{Path, PathBuf};

mod backends;

#[cfg(not(any(feature = "shaderc", feature = "naga")))]
compile_error!("no compiler backend enabled; please specify at least one of \
    the following input source features: `glsl`, `hlsl`, `wgsl`");

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream, Result as ParseResult, Error as ParseError};
use syn::{parse_macro_input, Ident, LitStr, Token};

#[derive(Clone, Copy)]
enum InputSourceLanguage {
    Unknown,
    Glsl,
    Hlsl,
    Wgsl,
}
#[derive(Clone, Copy)]
enum TargetSpirvVersion {
    Spirv1_0,
    #[allow(dead_code)]
    Spirv1_1,
    #[allow(dead_code)]
    Spirv1_2,
    Spirv1_3,
    #[allow(dead_code)]
    Spirv1_4,
    Spirv1_5,
}
#[derive(Clone, Copy)]
enum TargetEnvironmentType {
    Vulkan,
    OpenGL,
    WebGpu,
}
#[derive(Clone, Copy)]
enum OptimizationLevel {
    MinSize,
    MaxPerformance,
    None,
}
#[derive(Clone, Copy)]
enum ShaderKind {
    Unknown,

    Vertex,
    TesselationControl,
    TesselationEvaluation,
    Geometry,
    Fragment,
    Compute,
    // Mesh Pipeline
    Mesh,
    Task,
    // Ray-tracing Pipeline
    RayGeneration,
    Intersection,
    AnyHit,
    ClosestHit,
    Miss,
    Callable,
}

struct ShaderCompilationConfig {
    lang: InputSourceLanguage,
    incl_dirs: Vec<PathBuf>,
    defs: Vec<(String, Option<String>)>,
    spv_ver: TargetSpirvVersion,
    env_ty: TargetEnvironmentType,
    entry: String,
    optim_lv: OptimizationLevel,
    debug: bool,
    kind: ShaderKind,
    auto_bind: bool,
    // Backend specific.
    #[cfg(feature = "naga")]
    y_flip: bool,
}
impl Default for ShaderCompilationConfig {
    fn default() -> Self {
        ShaderCompilationConfig {
            lang: InputSourceLanguage::Unknown,
            incl_dirs: Vec::new(),
            defs: Vec::new(),
            spv_ver: TargetSpirvVersion::Spirv1_0,
            env_ty: TargetEnvironmentType::Vulkan,
            entry: "main".to_owned(),
            optim_lv: OptimizationLevel::None,
            debug: true,
            kind: ShaderKind::Unknown,
            auto_bind: false,

            #[cfg(feature = "naga")]
            y_flip: true,
        }
    }
}

struct CompilationFeedback {
    spv: Vec<u32>,
    dep_paths: Vec<String>,
}
struct InlineShaderSource(CompilationFeedback);
struct IncludedShaderSource(CompilationFeedback);

#[inline]
fn get_base_dir() -> PathBuf {
    let base_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("`inline-spirv` can only be used in build time");
    PathBuf::from(base_dir)
}
#[inline]
fn parse_str(input: &mut ParseStream) -> ParseResult<String> {
    input.parse::<LitStr>()
        .map(|x| x.value())
}
#[inline]
fn parse_ident(input: &mut ParseStream) -> ParseResult<String> {
    input.parse::<Ident>()
        .map(|x| x.to_string())
}

fn parse_compile_cfg(
    input: &mut ParseStream
) -> ParseResult<ShaderCompilationConfig> {
    let mut cfg = ShaderCompilationConfig::default();
    while !input.is_empty() {
        use syn::Error;
        // Capture comma and collon; they are for readability.
        input.parse::<Token![,]>()?;
        let k = if let Ok(k) = input.parse::<Ident>() { k } else { break };
        match &k.to_string() as &str {
            "glsl" => cfg.lang = InputSourceLanguage::Glsl,
            "hlsl" => {
                cfg.lang = InputSourceLanguage::Hlsl;
                // HLSL might be illegal if optimization is disabled. Not sure,
                // `glslangValidator` said this.
                cfg.optim_lv = OptimizationLevel::MaxPerformance;
            },
            "wgsl" => cfg.lang = InputSourceLanguage::Wgsl,

            "vert" => cfg.kind = ShaderKind::Vertex,
            "tesc" => cfg.kind = ShaderKind::TesselationControl,
            "tese" => cfg.kind = ShaderKind::TesselationEvaluation,
            "geom" => cfg.kind = ShaderKind::Geometry,
            "frag" => cfg.kind = ShaderKind::Fragment,
            "comp" => cfg.kind = ShaderKind::Compute,
            "mesh" => cfg.kind = ShaderKind::Mesh,
            "task" => cfg.kind = ShaderKind::Task,
            "rgen" => cfg.kind = ShaderKind::RayGeneration,
            "rint" => cfg.kind = ShaderKind::Intersection,
            "rahit" => cfg.kind = ShaderKind::AnyHit,
            "rchit" => cfg.kind = ShaderKind::ClosestHit,
            "rmiss" => cfg.kind = ShaderKind::Miss,
            "rcall" => cfg.kind = ShaderKind::Callable,

            "I" => {
                cfg.incl_dirs.push(PathBuf::from(parse_str(input)?))
            },
            "D" => {
                let k = parse_ident(input)?;
                let v = if input.parse::<Token![=]>().is_ok() {
                    Some(parse_str(input)?)
                } else { None };
                cfg.defs.push((k, v));
            },

            "entry" => {
                if input.parse::<Token![=]>().is_ok() {
                    cfg.entry = parse_str(input)?.to_owned();
                }
            }

            "min_size" => cfg.optim_lv = OptimizationLevel::MinSize,
            "max_perf" => cfg.optim_lv = OptimizationLevel::MaxPerformance,

            "no_debug" => cfg.debug = false,

            "vulkan" | "vulkan1_0" => {
                cfg.env_ty = TargetEnvironmentType::Vulkan;
                cfg.spv_ver = TargetSpirvVersion::Spirv1_0;
            },
            "vulkan1_1" => {
                cfg.env_ty = TargetEnvironmentType::Vulkan;
                cfg.spv_ver = TargetSpirvVersion::Spirv1_3;
            },
            "vulkan1_2" => {
                cfg.env_ty = TargetEnvironmentType::Vulkan;
                cfg.spv_ver = TargetSpirvVersion::Spirv1_5;
            },
            "opengl" | "opengl4_5" => {
                cfg.env_ty = TargetEnvironmentType::OpenGL;
                cfg.spv_ver = TargetSpirvVersion::Spirv1_0;
            },
            "webgpu" => {
                cfg.env_ty = TargetEnvironmentType::WebGpu;
                cfg.spv_ver = TargetSpirvVersion::Spirv1_0;
            }

            "auto_bind" => cfg.auto_bind = true,

            #[cfg(feature = "naga")]
            "no_y_flip" => cfg.y_flip = false,

            _ => return Err(Error::new(k.span(), "unsupported compilation parameter")),
        }
    }
    Ok(cfg)
}

fn compile(
    src: &str,
    path: Option<&str>,
    cfg: &ShaderCompilationConfig,
) -> Result<CompilationFeedback, String> {
    backends::naga::compile(src, path, cfg)
        .or_else(|_| backends::shaderc::compile(src, path, cfg))
}

impl Parse for IncludedShaderSource {
    fn parse(mut input: ParseStream) -> ParseResult<Self> {
        let path_lit = input.parse::<LitStr>()?;
        let path = Path::new(&get_base_dir())
            .join(&path_lit.value())
            .to_string_lossy()
            .to_string();
        let src = std::fs::read_to_string(&path)
            .map_err(|e| syn::Error::new(path_lit.span(), e))?;
        let cfg = parse_compile_cfg(&mut input)?;
        let feedback = compile(&src, Some(&path), &cfg)
            .map_err(|e| ParseError::new(input.span(), e))?;
        let rv = IncludedShaderSource(feedback);
        Ok(rv)
    }
}
impl Parse for InlineShaderSource {
    fn parse(mut input: ParseStream) -> ParseResult<Self> {
        let src = parse_str(&mut input)?;
        let cfg = parse_compile_cfg(&mut input)?;
        let feedback = compile(&src, None, &cfg)
            .map_err(|e| ParseError::new(input.span(), e))?;
        let rv = InlineShaderSource(feedback);
        Ok(rv)
    }
}

fn gen_token_stream(feedback: CompilationFeedback) -> TokenStream {
    let CompilationFeedback { spv, dep_paths } = feedback;
    (quote! {
        {
            { #(let _ = include_bytes!(#dep_paths);)* }
            &[#(#spv),*]
        }
    }).into()
}

/// Compile inline shader source and embed the SPIR-V binary word sequence.
/// Returns a `&'static [u32]`.
#[proc_macro]
pub fn inline_spirv(tokens: TokenStream) -> TokenStream {
    let InlineShaderSource(feedback) = parse_macro_input!(tokens as InlineShaderSource);
    gen_token_stream(feedback)
}
/// Compile external shader source and embed the SPIR-V binary word sequence.
/// Returns a `&'static [u32]`.
#[proc_macro]
pub fn include_spirv(tokens: TokenStream) -> TokenStream {
    let IncludedShaderSource(feedback) = parse_macro_input!(tokens as IncludedShaderSource);
    gen_token_stream(feedback)
}
