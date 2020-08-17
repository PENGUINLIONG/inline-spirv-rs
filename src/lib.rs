use std::path::{Path, PathBuf};
use std::{env, fs, str};

use shaderc::{ShaderKind, SourceLanguage, OptimizationLevel, CompileOptions,
    TargetEnv};

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream, Result as ParseResult, Error as ParseError};
use syn::{parse_macro_input, Ident, LitStr, Token};

struct ShaderCompilationConfig {
    lang: SourceLanguage,
    kind: ShaderKind,
    incl_dirs: Vec<String>,
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
            incl_dirs: Vec::new(),
            defs: Vec::new(),
            entry: String::new(),
            optim_lv: OptimizationLevel::Zero,
            debug: true,
            auto_bind: false,
        }
    }
}

struct InlineShaderSource(Vec<u32>);
struct IncludedShaderSource(Vec<u32>);

#[inline]
fn get_base_dir() -> String {
    env::var("CARGO_MANIFEST_DIR")
        .expect("`inline-spirv` can only be used in build time")
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
        let k = input.parse::<Ident>()?;
        match &k.to_string() as &str {
            "glsl" => cfg.lang = SourceLanguage::GLSL,
            "hlsl" => cfg.lang = SourceLanguage::HLSL,

            "vert" => cfg.kind = ShaderKind::Vertex,
            "tesc" => cfg.kind = ShaderKind::TessControl,
            "tese" => cfg.kind = ShaderKind::TessEvaluation,
            "geom" => cfg.kind = ShaderKind::Geometry,
            "frag" => cfg.kind = ShaderKind::Fragment,
            "comp" => cfg.kind = ShaderKind::Compute,

            "I" => cfg.incl_dirs.push(parse_str(input)?),
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
) -> Result<Vec<u32>, String> {
    let base_dir = PathBuf::from(get_base_dir());
    let mut opt = CompileOptions::new().unwrap();
    opt.set_target_env(TargetEnv::Vulkan, 0);
    opt.set_source_language(cfg.lang);
    opt.set_include_callback(move |name, ty, src_path, _depth| {
        use shaderc::{IncludeType, ResolvedInclude};
        let path = match ty {
            IncludeType::Relative => {
                let cur_dir = Path::new(src_path).parent().unwrap();
                cur_dir.join(name)
            },
            IncludeType::Standard => base_dir.join(name),
        };

        // FIXME: (penguinliong)
        let incl = ResolvedInclude {
            resolved_name: path.to_string_lossy().to_string(),
            content: fs::read_to_string(path).unwrap(),
        };
        Ok(incl)
    });
    opt.set_auto_bind_uniforms(cfg.auto_bind);
    for (k, v) in cfg.defs.iter() {
        opt.add_macro_definition(&k, v.as_ref().map(|x| x.as_ref()));
    }
    opt.set_optimization_level(cfg.optim_lv);
    if cfg.debug {
        opt.set_generate_debug_info();
    }


    let mut compiler = shaderc::Compiler::new().unwrap();
    let path = path
        .unwrap_or("<inline source>");
    let out = compiler
        .compile_into_spirv(src, cfg.kind, &path, "main", Some(&opt))
        .map_err(|e| e.to_string())?;
    if out.get_num_warnings() != 0 {
        return Err(out.get_warning_messages());
    }
    let rv = out.as_binary().into();
    Ok(rv)
}

impl Parse for IncludedShaderSource {
    fn parse(mut input: ParseStream) -> ParseResult<Self> {
        let path_lit = input.parse::<LitStr>()?;
        let path = Path::new(&get_base_dir())
            .join(&path_lit.value())
            .to_string_lossy()
            .to_string();
        let src = fs::read_to_string(&path)
            .map_err(|e| syn::Error::new(path_lit.span(), e))?;
        let cfg = parse_compile_cfg(&mut input)?;
        let spv = compile(&src, Some(&path), &cfg)
            .map_err(|e| ParseError::new(input.span(), e))?;
        let rv = IncludedShaderSource(spv);
        Ok(rv)
    }
}
impl Parse for InlineShaderSource {
    fn parse(mut input: ParseStream) -> ParseResult<Self> {
        let src = parse_str(&mut input)?;
        let cfg = parse_compile_cfg(&mut input)?;
        let spv = compile(&src, None, &cfg)
            .map_err(|e| ParseError::new(input.span(), e))?;
        let rv = InlineShaderSource(spv);
        Ok(rv)
    }
}

#[proc_macro]
pub fn inline_spirv(tokens: TokenStream) -> TokenStream {
    let InlineShaderSource(spv) = parse_macro_input!(tokens as InlineShaderSource);
    let expanded = quote! { { &[#(#spv),*] } };
    TokenStream::from(expanded)
}
#[proc_macro]
pub fn include_spirv(tokens: TokenStream) -> TokenStream {
    let IncludedShaderSource(spv) = parse_macro_input!(tokens as IncludedShaderSource);
    let expanded = quote! { { &[#(#spv),*] } };
    TokenStream::from(expanded)
}
