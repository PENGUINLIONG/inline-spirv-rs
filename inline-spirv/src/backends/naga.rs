#[allow(unused_imports)]
use crate::{CompilationFeedback, InputSourceLanguage, OptimizationLevel,
    TargetEnvironmentType, TargetSpirvVersion, ShaderKind,
    ShaderCompilationConfig};

#[cfg(feature = "naga")]
pub(crate) fn compile(
    src: &str,
    _path: Option<&str>,
    cfg: &ShaderCompilationConfig,
) -> Result<CompilationFeedback, String> {
    use naga::{
        valid::{ValidationFlags, Validator, Capabilities},
        back::spv::WriterFlags,
    };

    let module = match cfg.lang {
        InputSourceLanguage::Unknown => naga::front::wgsl::parse_str(src),
        InputSourceLanguage::Wgsl => naga::front::wgsl::parse_str(src),
        _ => return Err("unsupported source language".to_owned()),
    };
    let module = module.map_err(|e| e.emit_to_string(src))?;
    let mut opts = naga::back::spv::Options::default();
    match (cfg.env_ty, cfg.spv_ver) {
        (TargetEnvironmentType::Vulkan, TargetSpirvVersion::Spirv1_0) => {
            opts.lang_version = (1, 0);
        },
        (TargetEnvironmentType::Vulkan, TargetSpirvVersion::Spirv1_3) => {
            opts.lang_version = (1, 3);
        },
        (TargetEnvironmentType::Vulkan, TargetSpirvVersion::Spirv1_5) => {
            opts.lang_version = (1, 5);
        },
        (TargetEnvironmentType::OpenGL, TargetSpirvVersion::Spirv1_0) => {
            opts.lang_version = (1, 0);
        },
        (TargetEnvironmentType::WebGpu, TargetSpirvVersion::Spirv1_0) => {
            opts.lang_version = (1, 0);
        },
        _ => return Err("unsupported target".to_owned()),
    };
    if cfg.debug {
        opts.flags.insert(WriterFlags::DEBUG);
    } else {
        opts.flags.remove(WriterFlags::DEBUG);
    }
    if cfg.y_flip {
        opts.flags.insert(WriterFlags::ADJUST_COORDINATE_SPACE);
    } else {
        opts.flags.remove(WriterFlags::ADJUST_COORDINATE_SPACE);
    }

    // Attempt to validate WGSL, error if invalid
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .map_err(|e| format!("{:?}", e))?;
    let spv = naga::back::spv::write_vec(&module, &info, &opts)
        .map_err(|e| format!("{:?}", e))?;
    let feedback = CompilationFeedback {
        spv,
        dep_paths: Vec::new()
    };
    Ok(feedback)
}

#[cfg(not(feature = "naga"))]
pub(crate) fn compile(
    _: &str,
    _: Option<&str>,
    _: &ShaderCompilationConfig,
) -> Result<CompilationFeedback, String> {
    Err("naga backend is not enabled".to_owned())
}
