use super::traits::Backend;
use crate::{
    errors::{Error, FutilResult},
    ir,
};
use serde::Serialize;

#[derive(Default)]
pub struct XilinxXmlBackend;

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

    fn validate(_ctx: &ir::Context) -> FutilResult<()> {
        Ok(())
    }

    fn link_externs(
        _prog: &ir::Context,
        _write: &mut crate::utils::OutputFile,
    ) -> FutilResult<()> {
        Ok(())
    }

    fn emit(
        prog: &ir::Context,
        file: &mut crate::utils::OutputFile,
    ) -> FutilResult<()> {
        let _toplevel = prog
            .components
            .iter()
            .find(|comp| comp.attributes.has("toplevel"))
            .ok_or_else(|| Error::Misc("no toplevel".to_string()))?;

        let root = Root {
            version_major: 1,
            version_minor: 6,
            kernel: Kernel {
                name: "Toplevel",
                language: "ip_c",
                vlnv: "capra.cs.cornell.edu:kernel:Toplevel:1.0",
                attributes: "",
                preferred_work_group_size_multiple: 0,
                work_group_size: 1,
                interrupt: true,
                hw_control_protocol: "ap_ctrl_hs",
                ports: vec![
                    Port {
                        name: "s_axi_control",
                        mode: "slave",
                        range: "0x1000",
                        data_width: 32,
                        port_type: "addressable",
                        base: "0x0",
                    },
                    // Port {
                    //     name: "m00_axi",
                    //     mode: "master",
                    //     range: "0xFFFFFFFFFFFFFFFF",
                    //     data_width: 64,
                    //     port_type: "addressable",
                    //     base: "0x0",
                    // },
                ]
                .into(),
                args: vec![Arg {
                    name: "timer",
                    address_qualifier: 0,
                    id: 0,
                    port: "s_axi_control",
                    size: "0x4",
                    offset: "0x010",
                    typ: "uint",
                    host_offset: "0x0",
                    host_size: "0x4",
                }]
                .into(),
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
