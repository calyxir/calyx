use std::fmt::Display;

use yxi::ProgramInterface;

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
}
