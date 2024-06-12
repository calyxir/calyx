use bad_calyx_builder::{finish_component, CalyxBuilder, CalyxComponent};
use calyx_ir::{Attributes, PortDef};
use std::{fmt::Display, path::PathBuf};
use yxi::{Memory, ProgramInterface};

const WRAPPER_COMPONENT_NAME: &str = "wrapper";

pub enum YXIParseError {
    WidthNotMultipleOf8,
    WidthNotPowerOf2,
    SizeNotPositive,
}

mod axi_prefix {
    pub const ADDRESS_WRITE: &str = "AW";
    pub const ADDRESS_READ: &str = "AR";
    pub const WRITE: &str = "W";
    pub const READ: &str = "R";
}

enum AXIDirection {
    Read,
    Write,
}

impl AXIDirection {
    fn address_prefix(&self) -> &str {
        match &self {
            Self::Read => axi_prefix::ADDRESS_READ,
            Self::Write => axi_prefix::ADDRESS_WRITE,
        }
    }

    fn data_prefix(&self) -> &str {
        match &self {
            Self::Read => axi_prefix::READ,
            Self::Write => axi_prefix::WRITE,
        }
    }
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
            } else if memory.total_size == 0 {
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
        )?;
        for memory in &self.yxi.memories {
            build_axi_interface(&mut builder, memory);
        }
        Ok(builder.finalize())
    }
}

fn build_axi_interface(builder: &mut CalyxBuilder, memory: &Memory) {}

fn build_address_channel<F>(
    builder: &mut CalyxBuilder,
    memory: &Memory,
    direction: AXIDirection,
    and_then: F,
) where
    F: FnOnce(&mut CalyxComponent<()>) -> (),
{
    let x = direction.address_prefix();
    let lowercased_x = x.to_lowercase();

    let inputs = [("ARESETn", 1), (&format!("{}READY", x), 1)];
    let outputs = [
        (&format!("{}VALID", x), 1),
        (&format!("{}ADDR", x), 64),
        (&format!("{}SIZE", x), 3),
        (&format!("{}LEN", x), 8),
        (&format!("{}BURST", x), 2),
        (&format!("{}PROT", x), 3),
    ];
    let ports = build_port_defs(&inputs, &outputs);
    let mut comp =
        new_component(builder, format!("m_{}_channel", lowercased_x), ports);

    let xvalid = comp.new_reg(format!("{}valid", lowercased_x), 1);
    let xhandshake_occured =
        comp.new_reg(format!("{}_handshake_occured", lowercased_x), 1);
    let curr_addr_axi = comp.with_calyx_builder(|b| {
        let mut reg = b.add_primitive("curr_addr_axi", "std_reg", &[64]);
        // reg.borrow_mut().set_reference(true);
        reg
    });
    let xlen = comp.new_reg(format!("{}len", lowercased_x), 8);

    and_then(&mut comp);
}

fn build_port_defs<S: ToString, T: ToString>(
    inputs: &[(S, u64)],
    outputs: &[(T, u64)],
) -> Vec<PortDef<u64>> {
    let mut ports = vec![];
    ports.extend(inputs.iter().map(|(name, width)| {
        PortDef::new(
            name.to_string(),
            *width,
            calyx_ir::Direction::Input,
            Attributes::default(),
        )
    }));
    ports.extend(outputs.iter().map(|(name, width)| {
        PortDef::new(
            name.to_string(),
            *width,
            calyx_ir::Direction::Output,
            Attributes::default(),
        )
    }));
    ports
}

fn new_component(
    builder: &mut CalyxBuilder,
    name: String,
    ports: Vec<PortDef<u64>>,
) -> CalyxComponent<()> {
    builder.register_component(name.clone(), ports);
    builder.start_component(name)
}
