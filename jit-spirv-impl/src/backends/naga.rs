use syn::Expr;

#[allow(unused_imports)]
use crate::{
    InputSourceLanguage, OptimizationLevel, ShaderCompilationConfig, ShaderKind,
    TargetEnvironmentType, TargetSpirvVersion,
};

pub(crate) fn generate_compile_code(
    _: &Expr,
    _: &ShaderCompilationConfig,
) -> Result<proc_macro::TokenStream, String> {
    Err("naga backend is not enabled".to_owned())
}
