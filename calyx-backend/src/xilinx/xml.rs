use crate::traits::Backend;
use calyx_ir as ir;
use calyx_utils::CalyxResult;
use serde::Serialize;

/// Backend that generates XML that Xilinx needs to define the address
/// space for a kernel.
#[derive(Default)]
pub struct XilinxXmlBackend;

/// The root element of the `kernel.xml` file that describes an `.xo` package for the
/// Xilinx toolchain, as documented [in the Vitis user guide][ug].
///
/// [ug]: https://docs.xilinx.com/r/en-US/ug1393-vitis-application-acceleration/RTL-Kernel-XML-File
#[derive(Serialize)]
#[serde(rename = "root")]
struct Root<'a> {
    #[serde(rename = "@versionMajor")]
    version_major: u64,
    #[serde(rename = "@versionMinor")]
    version_minor: u64,
    kernel: Kernel<'a>,
}

#[derive(Serialize)]
#[serde(rename = "kernel")]
struct Kernel<'a> {
    #[serde(rename = "@name")]
    name: &'a str,
    #[serde(rename = "@language")]
    language: &'a str,
    #[serde(rename = "@vlnv")]
    vlnv: &'a str,
    #[serde(rename = "@preferredWorkGroupSizeMultiple")]
    preferred_work_group_size_multiple: u64,
    #[serde(rename = "@workGroupSize")]
    work_group_size: u64,
    #[serde(rename = "@interrupt")]
    interrupt: bool,
    #[serde(rename = "@hwControlProtocol")]
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
#[serde(rename = "port")]
struct Port<'a> {
    #[serde(rename = "@name")]
    name: &'a str,
    #[serde(rename = "@mode")]
    mode: &'a str,
    #[serde(rename = "@range")]
    range: &'a str,
    #[serde(rename = "@dataWidth")]
    data_width: u64,
    #[serde(rename = "@portType")]
    port_type: &'a str,
    #[serde(rename = "@base")]
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
struct Arg<'a> {
    #[serde(rename = "@name")]
    name: &'a str,
    #[serde(rename = "@addressQualifier")]
    address_qualifier: u64,
    #[serde(rename = "@id")]
    id: u64,
    #[serde(rename = "@port")]
    port: &'a str,
    #[serde(rename = "@size")]
    size: &'a str,
    #[serde(rename = "@offset")]
    offset: &'a str,
    #[serde(rename = "@type")]
    typ: &'a str,
    #[serde(rename = "@hostOffset")]
    host_offset: &'a str,
    #[serde(rename = "@hostSize")]
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
        _write: &mut calyx_utils::OutputFile,
    ) -> CalyxResult<()> {
        Ok(())
    }

    fn emit(
        prog: &ir::Context,
        file: &mut calyx_utils::OutputFile,
    ) -> CalyxResult<()> {
        let toplevel = prog
            .components
            .iter()
            .find(|comp| comp.name == prog.entrypoint)
            .unwrap();

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
                cell_ref
                    .borrow()
                    .get_attribute(ir::BoolAttr::External)
                    .is_some()
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
                // Width should match the bus data width of memory modules
                // described in hardware, for example see
                // https://github.com/calyxir/calyx/blob/c2b12a0fe6b1ee3aaaae0c66e7c4619ee6c82614/src/backend/xilinx/toplevel.rs#L58
                data_width: 512,
                port_type: "addressable",
                base: "0x0",
            });
            args.push(Arg {
                name,
                address_qualifier: 1,
                id: (i + 1) as u64,
                port: axi_name,
                // XXX(nathanielnrn): This should probably be assigned dynamically
                // and not hardcoded, need to figure out where this comes from
                // One theory: this is an 8-byte pointer to our argument arrays
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
