extern crate proc_macro;
use std::path::{Path, PathBuf};
use shaderc::{ShaderKind, SourceLanguage, OptimizationLevel, CompileOptions,
    TargetEnv, Compiler};
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
            entry: String::new(),
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

            "vert" => cfg.kind = ShaderKind::Vertex,
            "tesc" => cfg.kind = ShaderKind::TessControl,
            "tese" => cfg.kind = ShaderKind::TessEvaluation,
            "geom" => cfg.kind = ShaderKind::Geometry,
            "frag" => cfg.kind = ShaderKind::Fragment,
            "comp" => cfg.kind = ShaderKind::Compute,

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
    opt.set_target_env(TargetEnv::Vulkan, 0);
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

#[proc_macro]
pub fn inline_spirv(tokens: TokenStream) -> TokenStream {
    let InlineShaderSource(feedback) = parse_macro_input!(tokens as InlineShaderSource);
    gen_token_stream(feedback)
}
#[proc_macro]
pub fn include_spirv(tokens: TokenStream) -> TokenStream {
    let IncludedShaderSource(feedback) = parse_macro_input!(tokens as IncludedShaderSource);
    gen_token_stream(feedback)
}
