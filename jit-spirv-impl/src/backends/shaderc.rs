use syn::Ident;
use quote::quote;

#[allow(unused_imports)]
use crate::{InputSourceLanguage, OptimizationLevel,
    TargetEnvironmentType, TargetSpirvVersion, ShaderKind,
    ShaderCompilationConfig};

#[cfg(feature = "shaderc")]
pub(crate) fn generate_compile_code(
    src: Ident,
    cfg: &ShaderCompilationConfig,
) -> Result<proc_macro2::TokenStream, String> {
    use proc_macro2::Span;
    use syn::LitStr;

    let lang = match cfg.lang {
        InputSourceLanguage::Unknown => quote!(
            ::jit_spirv::dep::shaderc::SourceLanguage::GLSL
        ),
        InputSourceLanguage::Glsl => quote!(
            ::jit_spirv::dep::shaderc::SourceLanguage::GLSL
        ),
        InputSourceLanguage::Hlsl => quote!(
            ::jit_spirv::dep::shaderc::SourceLanguage::HLSL
        ),
        _ => return Err("unsupported source language".to_owned()),
    };
    let (target_env, vulkan_version) = match (cfg.env_ty, cfg.spv_ver) {
        (TargetEnvironmentType::Vulkan, TargetSpirvVersion::Spirv1_0) => (
            quote!(::jit_spirv::dep::shaderc::TargetEnv::Vulkan),
            quote!(::jit_spirv::dep::shaderc::EnvVersion::Vulkan1_0),
        ),
        (TargetEnvironmentType::Vulkan, TargetSpirvVersion::Spirv1_3) => (
            quote!(::jit_spirv::dep::shaderc::TargetEnv::Vulkan),
            quote!(::jit_spirv::dep::shaderc::EnvVersion::Vulkan1_1),
        ),
        (TargetEnvironmentType::Vulkan, TargetSpirvVersion::Spirv1_5) => (
            quote!(::jit_spirv::dep::shaderc::TargetEnv::Vulkan),
            quote!(::jit_spirv::dep::shaderc::EnvVersion::Vulkan1_2),
        ),
        (TargetEnvironmentType::OpenGL, TargetSpirvVersion::Spirv1_0) => (
            quote!(::jit_spirv::dep::shaderc::TargetEnv::OpenGL),
            quote!(::jit_spirv::dep::shaderc::EnvVersion::OpenGL4_5),
        ),
        _ => return Err("unsupported target".to_owned()),
    };
    let auto_bind = if cfg.auto_bind { quote!(true) } else { quote!(false) };
    let optim_lv = match cfg.optim_lv {
        OptimizationLevel::None => quote!(
            ::jit_spirv::dep::shaderc::OptimizationLevel::Zero
        ),
        OptimizationLevel::MinSize => quote!(
            ::jit_spirv::dep::shaderc::OptimizationLevel::Size
        ),
        OptimizationLevel::MaxPerformance => quote!(
            ::jit_spirv::dep::shaderc::OptimizationLevel::Performance
        ),
    };
    let shader_kind = match cfg.kind {
        ShaderKind::Unknown               => quote!(::jit_spirv::dep::shaderc::ShaderKind::InferFromSource),
        ShaderKind::Vertex                => quote!(::jit_spirv::dep::shaderc::ShaderKind::DefaultVertex),
        ShaderKind::TesselationControl    => quote!(::jit_spirv::dep::shaderc::ShaderKind::DefaultTessControl),
        ShaderKind::TesselationEvaluation => quote!(::jit_spirv::dep::shaderc::ShaderKind::DefaultTessEvaluation),
        ShaderKind::Geometry              => quote!(::jit_spirv::dep::shaderc::ShaderKind::DefaultGeometry),
        ShaderKind::Fragment              => quote!(::jit_spirv::dep::shaderc::ShaderKind::DefaultFragment),
        ShaderKind::Compute               => quote!(::jit_spirv::dep::shaderc::ShaderKind::DefaultCompute),
        ShaderKind::Mesh                  => quote!(::jit_spirv::dep::shaderc::ShaderKind::DefaultMesh),
        ShaderKind::Task                  => quote!(::jit_spirv::dep::shaderc::ShaderKind::DefaultTask),
        ShaderKind::RayGeneration         => quote!(::jit_spirv::dep::shaderc::ShaderKind::DefaultRayGeneration),
        ShaderKind::Intersection          => quote!(::jit_spirv::dep::shaderc::ShaderKind::DefaultIntersection),
        ShaderKind::AnyHit                => quote!(::jit_spirv::dep::shaderc::ShaderKind::DefaultAnyHit),
        ShaderKind::ClosestHit            => quote!(::jit_spirv::dep::shaderc::ShaderKind::DefaultClosestHit),
        ShaderKind::Miss                  => quote!(::jit_spirv::dep::shaderc::ShaderKind::DefaultMiss),
        ShaderKind::Callable              => quote!(::jit_spirv::dep::shaderc::ShaderKind::DefaultCallable),
    };

    let incl_dirs = cfg.incl_dirs.iter()
        .map(|x| LitStr::new(x, Span::call_site()))
        .collect::<Vec<_>>();

    let defs = cfg.defs.iter()
        .map(|(k, v)| {
            let k = LitStr::new(k, Span::call_site());
            if let Some(v) = v {
                let v = LitStr::new(v, Span::call_site());
                quote!((#k, Some(#v)))
            } else {
                quote!((#k, None::<&str>))
            }
        })
        .collect::<Vec<_>>();

    let debug = if cfg.debug { quote!(true) } else { quote!(false) };

    let entry = LitStr::new(&cfg.entry, Span::call_site());

    let path = if let Some(path) = &cfg.path {
        let path = LitStr::new(&path, Span::call_site());
        quote!(Some(#path))
    } else {
        quote!(None::<&str>)
    };

    let generated_code = quote!({
        (|_: String| -> ::std::result::Result<::jit_spirv::CompilationFeedback, String> {
            let mut opt = ::jit_spirv::dep::shaderc::CompileOptions::new()
                .ok_or("cannot create `shaderc::CompileOptions`")?;
            opt.set_target_env(#target_env, #vulkan_version as u32);
            opt.set_source_language(#lang);
            opt.set_auto_bind_uniforms(#auto_bind);
            opt.set_optimization_level(#optim_lv);
            opt.set_include_callback(|name, ty, src_path, _depth| {
                use ::jit_spirv::dep::shaderc::{IncludeType, ResolvedInclude};
                let path = match ty {
                    IncludeType::Relative => {
                        let cur_dir = ::std::path::Path::new(src_path).parent()
                            .ok_or("the shader source is not living in a filesystem, but attempts to include a relative path")?;
                        cur_dir.join(name)
                    },
                    IncludeType::Standard => {
                        [".", #(#incl_dirs,)*].iter()
                            .find_map(|incl_dir| {
                                let path = ::std::path::PathBuf::from(incl_dir).join(name);
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
            for (k, v) in (&[#(#defs,)*] as &[(&str, Option<&str>)]).iter() {
                opt.add_macro_definition(&k, v.as_ref().map(|x| x.as_ref()));
            }
            if #debug {
                opt.set_generate_debug_info();
            }
        
            let dep_paths = std::cell::RefCell::new(Vec::new());
            let mut compiler = ::jit_spirv::dep::shaderc::Compiler::new().unwrap();
            let path = if let Some(path) = #path {
                dep_paths.borrow_mut().push(path.to_owned());
                path
            } else { "<inline>" };
            let out = compiler
                .compile_into_spirv(#src, #shader_kind, &path, #entry, Some(&opt))
                .map_err(|e| e.to_string())?;
            if out.get_num_warnings() != 0 {
                return Err(out.get_warning_messages());
            }
            let spv = out.as_binary().into();
            let feedback = ::jit_spirv::CompilationFeedback {
                spv,
                dep_paths: dep_paths.into_inner()
            };
            Ok(feedback)
        })
    });
    Ok(generated_code)
}

#[cfg(not(feature = "shaderc"))]
pub(crate) fn generate_compile_code(
    _: &Expr,
    _: &ShaderCompilationConfig,
) -> Result<proc_macro2::TokenStream, String> {
    Err("shaderc backend is not enabled".to_owned())
}
