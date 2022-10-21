#[allow(unused_imports)]
use crate::{CompilationFeedback, InputSourceLanguage, OptimizationLevel,
    TargetEnvironmentType, TargetSpirvVersion, ShaderKind,
    ShaderCompilationConfig};

#[cfg(feature = "shaderc")]
pub(crate) fn compile(
    src: &str,
    path: Option<&str>,
    cfg: &ShaderCompilationConfig,
) -> Result<CompilationFeedback, String> {
    use std::path::Path;
    use std::cell::RefCell;

    let lang = match cfg.lang {
        InputSourceLanguage::Unknown => shaderc::SourceLanguage::GLSL,
        InputSourceLanguage::Glsl => shaderc::SourceLanguage::GLSL,
        InputSourceLanguage::Hlsl => shaderc::SourceLanguage::HLSL,
        _ => return Err("unsupported source language".to_owned()),
    };
    let (target_env, vulkan_version) = match (cfg.env_ty, cfg.spv_ver) {
        (TargetEnvironmentType::Vulkan, TargetSpirvVersion::Spirv1_0) => (
            shaderc::TargetEnv::Vulkan,
            shaderc::EnvVersion::Vulkan1_0,
        ),
        (TargetEnvironmentType::Vulkan, TargetSpirvVersion::Spirv1_3) => (
            shaderc::TargetEnv::Vulkan,
            shaderc::EnvVersion::Vulkan1_1,
        ),
        (TargetEnvironmentType::Vulkan, TargetSpirvVersion::Spirv1_5) => (
            shaderc::TargetEnv::Vulkan,
            shaderc::EnvVersion::Vulkan1_2,
        ),
        (TargetEnvironmentType::OpenGL, TargetSpirvVersion::Spirv1_0) => (
            shaderc::TargetEnv::OpenGL,
            shaderc::EnvVersion::OpenGL4_5,
        ),
        _ => return Err("unsupported target".to_owned()),
    };
    let optim_lv = match cfg.optim_lv {
        OptimizationLevel::None => shaderc::OptimizationLevel::Zero,
        OptimizationLevel::MinSize => shaderc::OptimizationLevel::Size,
        OptimizationLevel::MaxPerformance => shaderc::OptimizationLevel::Performance,
    };
    let shader_kind = match cfg.kind {
        ShaderKind::Unknown               => shaderc::ShaderKind::InferFromSource,
        ShaderKind::Vertex                => shaderc::ShaderKind::DefaultVertex,
        ShaderKind::TesselationControl    => shaderc::ShaderKind::DefaultTessControl,
        ShaderKind::TesselationEvaluation => shaderc::ShaderKind::DefaultTessEvaluation,
        ShaderKind::Geometry              => shaderc::ShaderKind::DefaultGeometry,
        ShaderKind::Fragment              => shaderc::ShaderKind::DefaultFragment,
        ShaderKind::Compute               => shaderc::ShaderKind::DefaultCompute,
        ShaderKind::Mesh                  => shaderc::ShaderKind::DefaultMesh,
        ShaderKind::Task                  => shaderc::ShaderKind::DefaultTask,
        ShaderKind::RayGeneration         => shaderc::ShaderKind::DefaultRayGeneration,
        ShaderKind::Intersection          => shaderc::ShaderKind::DefaultIntersection,
        ShaderKind::AnyHit                => shaderc::ShaderKind::DefaultAnyHit,
        ShaderKind::ClosestHit            => shaderc::ShaderKind::DefaultClosestHit,
        ShaderKind::Miss                  => shaderc::ShaderKind::DefaultMiss,
        ShaderKind::Callable              => shaderc::ShaderKind::DefaultCallable,
    };

    let mut opt = shaderc::CompileOptions::new()
        .ok_or("cannot create `shaderc::CompileOptions`")?;
    opt.set_target_env(target_env, vulkan_version as u32);
    opt.set_source_language(lang);
    opt.set_auto_bind_uniforms(cfg.auto_bind);
    opt.set_optimization_level(optim_lv);
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

    let dep_paths = RefCell::new(Vec::new());
    let mut compiler = shaderc::Compiler::new().unwrap();
    let path = if let Some(path) = path {
        dep_paths.borrow_mut().push(path.to_owned());
        path
    } else { "<inline>" };
    let out = compiler
        .compile_into_spirv(src, shader_kind, &path, &cfg.entry, Some(&opt))
        .map_err(|e| e.to_string())?;
    if out.get_num_warnings() != 0 {
        return Err(out.get_warning_messages());
    }
    let spv = out.as_binary().into();
    let feedback = CompilationFeedback {
        spv,
        dep_paths: dep_paths.into_inner()
    };
    Ok(feedback)
}

#[cfg(not(feature = "shaderc"))]
pub(crate) fn compile(
    _: &str,
    _: Option<&str>,
    _: &ShaderCompilationConfig,
) -> Result<CompilationFeedback, String> {
    Err("shaderc backend is not enabled".to_owned())
}
