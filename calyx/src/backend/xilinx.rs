use super::{axi, traits::Backend};
use crate::{
    errors::{Error, FutilResult},
    ir,
};
use vast::v05::ast as v;

#[derive(Default)]
pub struct XilinxInterfaceBackend;

impl Backend for XilinxInterfaceBackend {
    fn name(&self) -> &'static str {
        "xilinx-axi"
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
        let toplevel = prog
            .components
            .iter()
            .find(|comp| comp.attributes.has("toplevel"))
            .ok_or_else(|| Error::Misc("no toplevel".to_string()))?;
        let memories = external_memories(toplevel);

        let mut module = v::Module::new("Control_axi");
        super::axi::axi(&mut module, 12, 32, &memories);

        write!(
            file.get_write(),
            "{}\n{}",
            top_level(12, 32, &memories),
            module
        )?;

        Ok(())
    }
}

fn external_memories(comp: &ir::Component) -> Vec<String> {
    // find external memories
    comp.cells
        .iter()
        .filter(|cell_ref| {
            matches!(cell_ref.borrow().get_attribute("external"), Some(&1))
        })
        .map(|cell_ref| cell_ref.borrow().name.to_string())
        .collect()
}

fn top_level(
    address_width: u64,
    data_width: u64,
    memories: &[String],
) -> v::Module {
    let mut module = v::Module::new("Toplevel");

    // add system signals
    module.add_input("ap_clk", 1);
    module.add_output("ap_interrupt", 1);

    // axi control signals
    let axi4 = axi::Axi4Lite::new(address_width, data_width, "s_axi_control_");
    axi4.add_ports_to(&mut module);

    // TODO: axi master interfaces

    // wires
    module.add_stmt(v::Decl::new_wire("ap_start", 1));
    module.add_stmt(v::Decl::new_wire("ap_done", 1));
    module.add_stmt(v::Decl::new_wire("timeout", 32));
    for mem in memories {
        module.add_stmt(v::Decl::new_wire(mem, 64));
    }

    // TODO: have real interrupt support
    module.add_stmt(v::Parallel::Assign(
        "ap_interrupt".into(),
        v::Expr::new_ulit_bin(1, "0"),
    ));

    // instantiate control interface
    let mut control_instance =
        v::Instance::new("inst_control_axi", "Control_axi");
    control_instance.connect("ACLK", "ap_clk");
    control_instance.connect("ARESET", v::Expr::new_ulit_bin(1, "0"));
    for mem in memories {
        control_instance.connect_ref(mem, mem);
    }
    control_instance.connect("ap_start", "ap_start");
    control_instance.connect("ap_done", "ap_done");
    control_instance.connect("timeout", "timeout");
    control_instance.connect("ARVALID", "s_axi_control_ARVALID");
    control_instance.connect("ARREADY", "s_axi_control_ARREADY");
    control_instance.connect("ARADDR", "s_axi_control_ARADDR");
    control_instance.connect("RREADY", "s_axi_control_RREADY");
    control_instance.connect("RVALID", "s_axi_control_RVALID");
    control_instance.connect("RDATA", "s_axi_control_RDATA");
    control_instance.connect("RRESP", "s_axi_control_RRESP");
    control_instance.connect("AWVALID", "s_axi_control_AWVALID");
    control_instance.connect("AWREADY", "s_axi_control_AWREADY");
    control_instance.connect("AWADDR", "s_axi_control_AWADDR");
    control_instance.connect("WVALID", "s_axi_control_WVALID");
    control_instance.connect("WREADY", "s_axi_control_WREADY");
    control_instance.connect("WDATA", "s_axi_control_WDATA");
    control_instance.connect("BREADY", "s_axi_control_BREADY");
    control_instance.connect("BVALID", "s_axi_control_BVALID");
    control_instance.connect("BRESP", "s_axi_control_BRESP");
    module.add_instance(control_instance);

    // add timeout counter
    module.add_decl(v::Decl::new_reg("counter", 32));
    let mut always_counter = v::ParallelProcess::new_always();
    always_counter.set_event(v::Sequential::new_posedge("ap_clk"));
    let mut reset_if = v::SequentialIfElse::new("ap_start".into());
    reset_if.add_seq(v::Sequential::new_nonblk_assign(
        "counter".into(),
        v::Expr::new_add("counter".into(), v::Expr::new_ulit_dec(32, "1")),
    ));
    reset_if.set_else(v::Sequential::new_nonblk_assign(
        "counter".into(),
        v::Expr::new_ulit_dec(32, "0"),
    ));
    always_counter.add_seq(reset_if.into());
    module.add_stmt(always_counter);
    module.add_stmt(v::Parallel::Assign(
        "ap_done".into(),
        v::Expr::new_gt("counter", "timeout"),
    ));

    module
}
