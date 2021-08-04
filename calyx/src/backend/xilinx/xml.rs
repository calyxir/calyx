use crate::{
    backend::traits::Backend,
    errors::{CalyxResult, Error},
    ir,
};
use serde::Serialize;

/// Backend that generates XML that Xilinx needs to define the address
/// space for a kernel.
#[derive(Default)]
pub struct XilinxXmlBackend;

// TODO(rachit): Document the Xilinx manual used to define this overall XML
// structure.
#[derive(Serialize)]
#[serde(rename = "root", rename_all = "camelCase")]
struct Root<'a> {
    version_major: u64,
    version_minor: u64,
    kernel: Kernel<'a>,
}

#[derive(Serialize)]
#[serde(rename = "kernel", rename_all = "camelCase")]
struct Kernel<'a> {
    name: &'a str,
    language: &'a str,
    vlnv: &'a str,
    attributes: &'a str,
    preferred_work_group_size_multiple: u64,
    work_group_size: u64,
    interrupt: bool,
    hw_control_protocol: &'a str,
    ports: Ports<'a>,
    args: Args<'a>,
}

#[derive(Serialize)]
struct Ports<'a> {
    port: Vec<Port<'a>>,
}
impl<'a> From<Vec<Port<'a>>> for Ports<'a> {
    fn from(port: Vec<Port<'a>>) -> Self {
        Ports { port }
    }
}

#[derive(Serialize)]
#[serde(rename = "port", rename_all = "camelCase")]
struct Port<'a> {
    name: &'a str,
    mode: &'a str,
    range: &'a str,
    data_width: u64,
    port_type: &'a str,
    base: &'a str,
}

#[derive(Serialize)]
struct Args<'a> {
    arg: Vec<Arg<'a>>,
}
impl<'a> From<Vec<Arg<'a>>> for Args<'a> {
    fn from(arg: Vec<Arg<'a>>) -> Self {
        Args { arg }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Arg<'a> {
    name: &'a str,
    address_qualifier: u64,
    id: u64,
    port: &'a str,
    size: &'a str,
    offset: &'a str,
    #[serde(rename = "type")]
    typ: &'a str,
    host_offset: &'a str,
    host_size: &'a str,
}

impl Backend for XilinxXmlBackend {
    fn name(&self) -> &'static str {
        "xilinx-xml"
    }

    fn validate(_ctx: &ir::Context) -> CalyxResult<()> {
        Ok(())
    }

    fn link_externs(
        _prog: &ir::Context,
        _write: &mut crate::utils::OutputFile,
    ) -> CalyxResult<()> {
        Ok(())
    }

    fn emit(
        prog: &ir::Context,
        file: &mut crate::utils::OutputFile,
    ) -> CalyxResult<()> {
        let toplevel = prog
            .components
            .iter()
            .find(|comp| comp.attributes.has("toplevel"))
            .ok_or_else(|| Error::Misc("no toplevel".to_string()))?;

        let mut ports = vec![Port {
            name: "s_axi_control",
            mode: "slave",
            range: "0x1000",
            data_width: 32,
            port_type: "addressable",
            base: "0x0",
        }];

        let mut args = vec![Arg {
            name: "timeout",
            address_qualifier: 0,
            id: 0,
            port: "s_axi_control",
            size: "0x4",
            offset: "0x010",
            typ: "uint",
            host_offset: "0x0",
            host_size: "0x4",
        }];

        let memories: Vec<(String, String)> = toplevel
            .cells
            .iter()
            .filter(|cell_ref| {
                matches!(cell_ref.borrow().get_attribute("external"), Some(&1))
            })
            .enumerate()
            .map(|(i, cell_ref)| {
                (cell_ref.borrow().name().to_string(), format!("m{}_axi", i))
            })
            .collect();
        // make the lifetime of the &str long enough
        let memories_ref: Vec<(&str, &str)> = memories
            .iter()
            .map(|(x, y)| (x.as_ref(), y.as_ref()))
            .collect();
        let offsets: Vec<String> = (0..memories.len())
            .map(|i| format!("{:#x}", 0x18 + (8 * i)))
            .collect();

        for (i, (name, axi_name)) in memories_ref.iter().enumerate() {
            ports.push(Port {
                name: axi_name,
                mode: "master",
                range: "0xFFFFFFFFFFFFFFFF",
                data_width: 64,
                port_type: "addressable",
                base: "0x0",
            });
            args.push(Arg {
                name,
                address_qualifier: 1,
                id: (i + 1) as u64,
                port: axi_name,
                size: "0x8",
                offset: &offsets[i],
                typ: "int*",
                host_offset: "0x0",
                host_size: "0x8",
            });
        }

        let root = Root {
            version_major: 1,
            version_minor: 6,
            kernel: Kernel {
                name: "Toplevel",
                language: "ip_c",
                // XXX(rachit): This hardcoding seems bad.
                vlnv: "capra.cs.cornell.edu:kernel:Toplevel:1.0",
                attributes: "",
                preferred_work_group_size_multiple: 0,
                work_group_size: 1,
                interrupt: false,
                hw_control_protocol: "ap_ctrl_hs",
                ports: ports.into(),
                args: args.into(),
            },
        };
        write!(
            file.get_write(),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}",
            quick_xml::se::to_string(&root).expect("XML Serialization failed")
        )?;

        Ok(())
    }
}
