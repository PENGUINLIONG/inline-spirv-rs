use syn::Expr;
use quote::quote;

#[allow(unused_imports)]
use crate::{
    InputSourceLanguage,
    OptimizationLevel,
    ShaderCompilationConfig,
    ShaderKind,
    TargetEnvironmentType,
    TargetSpirvVersion,
};

#[cfg(feature = "naga")]
pub(crate) fn generate_compile_code(
    src: &Expr,
    cfg: &ShaderCompilationConfig
) -> Result<proc_macro2::TokenStream, String> {
    match cfg.lang {
        InputSourceLanguage::Unknown => {},
        InputSourceLanguage::Wgsl => {},
        _ => return Err("unsupported source language".to_owned()),
    }

    let lang_version = match (cfg.env_ty, cfg.spv_ver) {
        (TargetEnvironmentType::Vulkan, TargetSpirvVersion::Spirv1_0) => quote!((1, 0)),
        (TargetEnvironmentType::Vulkan, TargetSpirvVersion::Spirv1_3) => quote!((1, 3)),
        (TargetEnvironmentType::Vulkan, TargetSpirvVersion::Spirv1_5) => quote!((1, 5)),
        (TargetEnvironmentType::OpenGL, TargetSpirvVersion::Spirv1_0) => quote!((1, 0)),
        (TargetEnvironmentType::WebGpu, TargetSpirvVersion::Spirv1_0) => quote!((1, 0)),
        _ => {
            return Err("unsupported target".to_owned());
        }
    };
    let writer_flags = {
        let mut out = quote!(::jit_spirv::dep::naga::back::spv::WriterFlags::empty());
        if cfg.debug {
            out.extend(quote!(| ::jit_spirv::dep::naga::back::spv::WriterFlags::DEBUG));
        }
        if cfg.y_flip {
            out.extend(
                quote!(| ::jit_spirv::dep::naga::back::spv::WriterFlags::ADJUST_COORDINATE_SPACE)
            );
        }
        out
    };

    let generated_code =
        quote!({
        (|_: String| {
            let mut opts = ::jit_spirv::dep::naga::back::spv::Options::default();
            opts.lang_version = #lang_version;
            opts.flags = #writer_flags;
            let module = ::jit_spirv::dep::naga::front::wgsl::parse_str(#src)
                .map_err(|e| e.emit_to_string(#src))?;
            let info = ::jit_spirv::dep::naga::valid::Validator::new(
                ::jit_spirv::dep::naga::valid::ValidationFlags::all(),
                ::jit_spirv::dep::naga::valid::Capabilities::all())
                .validate(&module)
                .map_err(|e| format!("{:?}", e))?;
            let spv = ::jit_spirv::dep::naga::back::spv::write_vec(&module, &info, &opts)
                .map_err(|e| format!("{:?}", e))?;
            let feedback = ::jit_spirv::CompilationFeedback {
                spv,
                dep_paths: Vec::new(),
            };
            Ok(feedback)
        })
    });
    Ok(generated_code)
}

#[cfg(not(feature = "naga"))]
pub(crate) fn generate_compile_code(
    _: &Expr,
    _: &ShaderCompilationConfig
) -> Result<proc_macro2::TokenStream, String> {
    Err("naga backend is not enabled".to_owned())
}