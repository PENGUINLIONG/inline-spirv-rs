//! # inline-spirv
//!
//!
//! The first string is always your shader path or the source code, depending on
//! the macro you use. Other following parameters give you finer control over
//! the compilation process:
//!
//! `inline-spirv` currently support two source languages:
//!
//! - `glsl`: The shader source is in GLSL;
//! - `hlsl`: The shader source is in HLSL.
//!
//! And the following shader stages:
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
//! You can also specify the entry function name (`main` by default):
//!
//! ```ignore
//! include_spirv!("path/to/shader.hlsl", hlsl, vert, entry="very_main");
//! ```
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
//! To decide how much you want the SPIR-V to be optimized:
//!
//! - `min_size`: Optimize for the minimal output size;
//! - `max_perf`: Optimize for the best performance;
//! - `no_debug`: Strip off all the debug information (don't do this if you want
//! to reflect the SPIR-V and get variable names).
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
//!
//! Of course once you started to use macro is basically means that you are
//! getting so dynamic that this little crate might not be enough. Then it might
//! be a good time to build your own shader compilation pipeline!
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
use shaderc::{ShaderKind, SourceLanguage, OptimizationLevel, CompileOptions,
    TargetEnv, Compiler, EnvVersion};
use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream, Result as ParseResult, Error as ParseError};
use syn::{parse_macro_input, Ident, LitStr, Token};

struct ShaderCompilationConfig {
    lang: SourceLanguage,
    kind: ShaderKind,
    incl_dirs: Vec<PathBuf>,
    defs: Vec<(String, Option<String>)>,
    entry: String,
    vulkan_version: EnvVersion,
    target_env: TargetEnv,
    optim_lv: OptimizationLevel,
    debug: bool,
    auto_bind: bool,
}
impl Default for ShaderCompilationConfig {
    fn default() -> ShaderCompilationConfig {
        ShaderCompilationConfig {
            lang: SourceLanguage::GLSL,
            kind: ShaderKind::InferFromSource,
            incl_dirs: vec![get_base_dir()],
            defs: Vec::new(),
            entry: "main".to_owned(),
            vulkan_version: EnvVersion::Vulkan1_0,
            target_env: TargetEnv::Vulkan,
            optim_lv: OptimizationLevel::Zero,
            debug: true,
            auto_bind: false,
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
            "glsl" => cfg.lang = SourceLanguage::GLSL,
            "hlsl" => {
                cfg.lang = SourceLanguage::HLSL;
                // HLSL might be illegal if optimization is disabled. Not sure,
                // `glslangValidator` said this.
                cfg.optim_lv = OptimizationLevel::Performance;
            },

            "vert" => cfg.kind = ShaderKind::DefaultVertex,
            "tesc" => cfg.kind = ShaderKind::DefaultTessControl,
            "tese" => cfg.kind = ShaderKind::DefaultTessEvaluation,
            "geom" => cfg.kind = ShaderKind::DefaultGeometry,
            "frag" => cfg.kind = ShaderKind::DefaultFragment,
            "comp" => cfg.kind = ShaderKind::DefaultCompute,
            "mesh" => cfg.kind = ShaderKind::DefaultMesh,
            "task" => cfg.kind = ShaderKind::DefaultTask,
            "rgen" => cfg.kind = ShaderKind::DefaultRayGeneration,
            "rint" => cfg.kind = ShaderKind::DefaultIntersection,
            "rahit" => cfg.kind = ShaderKind::DefaultAnyHit,
            "rchit" => cfg.kind = ShaderKind::DefaultClosestHit,
            "rmiss" => cfg.kind = ShaderKind::DefaultMiss,
            "rcall" => cfg.kind = ShaderKind::DefaultCallable,

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

            "min_size" => cfg.optim_lv = OptimizationLevel::Size,
            "max_perf" => cfg.optim_lv = OptimizationLevel::Performance,

            "no_debug" => cfg.debug = false,
            "auto_bind" => cfg.auto_bind = true,

            "vulkan1_0" => cfg.vulkan_version = EnvVersion::Vulkan1_0,
            "vulkan1_1" => cfg.vulkan_version = EnvVersion::Vulkan1_1,
            "vulkan1_2" => cfg.vulkan_version = EnvVersion::Vulkan1_2,
            "opengl4_5" => {
                cfg.vulkan_version = EnvVersion::OpenGL4_5;
                cfg.target_env = TargetEnv::OpenGL;
            },

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
    use std::cell::RefCell;
    let dep_paths = RefCell::new(Vec::new());

    let mut opt = CompileOptions::new()
        .ok_or("cannot create `shaderc::CompileOptions`")?;
    opt.set_target_env(cfg.target_env, cfg.vulkan_version as u32);
    opt.set_source_language(cfg.lang);
    opt.set_auto_bind_uniforms(cfg.auto_bind);
    opt.set_optimization_level(cfg.optim_lv);
    opt.set_include_callback(|name, ty, src_path, _depth| {
        use shaderc::{IncludeType, ResolvedInclude};
        let path = match ty {
            IncludeType::Relative => {
                let cur_dir = Path::new(src_path).parent()
                    .ok_or("the shader source is not living in a filesystem, but attempts to include a relative path")?;
                cur_dir.join(name)
            },
            IncludeType::Standard => {
                cfg.incl_dirs.iter()
                    .find_map(|incl_dir| {
                        let path = incl_dir.join(name);
                        if path.exists() { Some(path) } else { None }
                    })
                    .ok_or(format!("cannot find \"{}\" in include directories", name))?
            },
        };

        let path_lit = path.to_string_lossy().to_string();
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read from \"{}\": {}", path_lit, e.to_string()))?;
        let incl = ResolvedInclude { resolved_name: path_lit, content };
        Ok(incl)
    });
    for (k, v) in cfg.defs.iter() {
        opt.add_macro_definition(&k, v.as_ref().map(|x| x.as_ref()));
    }
    if cfg.debug {
        opt.set_generate_debug_info();
    }

    let mut compiler = Compiler::new().unwrap();
    let path = if let Some(path) = path {
        dep_paths.borrow_mut().push(path.to_owned());
        path
    } else { "<inline>" };
    let out = compiler
        .compile_into_spirv(src, cfg.kind, &path, &cfg.entry, Some(&opt))
        .map_err(|e| e.to_string())?;
    if out.get_num_warnings() != 0 {
        return Err(out.get_warning_messages());
    }
    let spv = out.as_binary().into();
    let feedback = CompilationFeedback { spv, dep_paths: dep_paths.into_inner() };
    Ok(feedback)
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
