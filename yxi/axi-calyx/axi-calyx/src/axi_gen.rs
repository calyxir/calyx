use bad_calyx_builder::CalyxBuilder;
use std::{fmt::Display, path::PathBuf};
use yxi::ProgramInterface;

const WRAPPER_COMPONENT_NAME: &str = "wrapper";

mod axi_prefix {
    const ADDRESS_WRITE: &str = "AW";
    const ADDRESS_READ: &str = "AR";
}

pub enum YXIParseError {
    WidthNotMultipleOf8,
    WidthNotPowerOf2,
    SizeNotPositive,
}

impl Display for YXIParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::WidthNotMultipleOf8 => {
                "Width must be a multiple of 8 to allow byte addressing to host"
            }
            Self::WidthNotPowerOf2 => {
                "Width must be a power of 2 to be correctly described by xSIZE"
            }
            Self::SizeNotPositive => "Memory size must be greater than 0",
        }
        .fmt(f)
    }
}

pub struct AXIGenerator {
    yxi: ProgramInterface,
}

impl AXIGenerator {
    pub fn parse(yxi: ProgramInterface) -> Result<AXIGenerator, YXIParseError> {
        for memory in &yxi.memories {
            if memory.data_width % 8 != 0 {
                Err(YXIParseError::WidthNotMultipleOf8)?;
            } else if memory.data_width & (memory.data_width - 1) != 0 {
                Err(YXIParseError::WidthNotPowerOf2)?;
            } else if memory.total_size <= 0 {
                Err(YXIParseError::SizeNotPositive)?;
            }
        }
        Ok(AXIGenerator { yxi })
    }

    pub fn yxi(&self) -> &ProgramInterface {
        &self.yxi
    }

    pub fn build(
        self,
        lib_path: PathBuf,
    ) -> calyx_utils::CalyxResult<calyx_ir::Context> {
        let mut builder = CalyxBuilder::new(
            None,
            lib_path,
            Some(WRAPPER_COMPONENT_NAME.into()),
            "_".into(),
        );
        Ok(builder.finalize())
    }
}
