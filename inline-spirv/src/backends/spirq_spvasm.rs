use spq_spvasm::{asm::Assembler, SpirvHeader};
use crate::{CompilationFeedback, InputSourceLanguage, ShaderCompilationConfig};

const SPIRV_VERSION_1_0: u32 = 0x0001_0000;
const SPIRV_VERSION_1_1: u32 = 0x0001_0100;
const SPIRV_VERSION_1_2: u32 = 0x0001_0200;
const SPIRV_VERSION_1_3: u32 = 0x0001_0300;
const SPIRV_VERSION_1_4: u32 = 0x0001_0400;
const SPIRV_VERSION_1_5: u32 = 0x0001_0500;
const SPIRV_VERSION_1_6: u32 = 0x0001_0600;

// TODO: (penguinliong) Get ourselves a generator ID.
const GENERATOR: u32 = 0;

pub(crate) fn compile(
    src: &str,
    path: Option<&str>,
    cfg: &ShaderCompilationConfig,
) -> Result<CompilationFeedback, String> {
    if cfg.lang != InputSourceLanguage::Spvasm {
        return Err("unsupported source language".to_owned());
    }

    let header = match cfg.spv_ver {
        crate::TargetSpirvVersion::Spirv1_0 => SpirvHeader::new(SPIRV_VERSION_1_0, GENERATOR),
        crate::TargetSpirvVersion::Spirv1_1 => SpirvHeader::new(SPIRV_VERSION_1_1, GENERATOR),
        crate::TargetSpirvVersion::Spirv1_2 => SpirvHeader::new(SPIRV_VERSION_1_2, GENERATOR),
        crate::TargetSpirvVersion::Spirv1_3 => SpirvHeader::new(SPIRV_VERSION_1_3, GENERATOR),
        crate::TargetSpirvVersion::Spirv1_4 => SpirvHeader::new(SPIRV_VERSION_1_4, GENERATOR),
        crate::TargetSpirvVersion::Spirv1_5 => SpirvHeader::new(SPIRV_VERSION_1_5, GENERATOR),
        crate::TargetSpirvVersion::Spirv1_6 => SpirvHeader::new(SPIRV_VERSION_1_6, GENERATOR),
    };

    Assembler::new().assemble(src, header)
        .map_err(|e| format!("failed to assemble SPIR-V: {}", e))
        .map(|binary| CompilationFeedback {
            spv: binary.into_words(),
            dep_paths: path.into_iter().map(|x| x.to_string()).collect(),
        })
}
