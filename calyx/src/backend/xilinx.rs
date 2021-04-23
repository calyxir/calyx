use super::traits::Backend;
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

        super::axi::axi();

        let module = emit_component(&toplevel);

        // write!(file.get_write(), "{}", module)?;

        Ok(())
    }
}

fn emit_component(comp: &ir::Component) -> v::Module {
    let mut module = v::Module::new("Control_axi_interface");

    // construct initial slave interface
    axi_slave_interface(&mut module, 12, 32);
    axi_local_signals(&mut module);

    // find external memories
    let external_cells = comp.cells.iter().filter(|cell_ref| {
        matches!(cell_ref.borrow().get_attribute("external"), Some(&1))
    });

    // add interface signals for memories
    for cell_ref in external_cells {
        let cell = cell_ref.borrow();
        let name = cell.name.as_ref();
        let width = 64;
        module.add_output(name, width);
        module.add_decl(v::Decl::new_reg(&format!("int_{}", name), width));
    }

    axi_write_fsm(&mut module);
    axi_read_fsm(&mut module);

    module
}

fn axi_slave_interface(
    module: &mut v::Module,
    addr_width: u64,
    data_width: u64,
) {
    // system signals
    module.add_input("ACLK", 1);
    module.add_input("ARESET", 1);
    module.add_input("ACLK_EN", 1);

    // write address channel
    module.add_input("AWVALID", 1);
    module.add_output("AWREADY", 1);
    module.add_input("AWADDR", addr_width);

    // write data channel
    module.add_input("WVALID", 1);
    module.add_output("WREADY", 1);
    module.add_input("WDATA", data_width);
    module.add_input("WSTRB", data_width / 8);

    // write response channel
    module.add_output("BVALID", 1);
    module.add_input("BREADY", 1);
    module.add_output("BRESP", 2);

    // read address channel
    module.add_input("ARVALID", 1);
    module.add_output("ARREADY", 1);
    module.add_input("ARADDR", addr_width);

    // read data channel
    module.add_output("RVALID", 1);
    module.add_input("RREADY", 1);
    module.add_output("RDATA", data_width);
    module.add_output("RRESP", 2);

    // control signals
    module.add_output("interrupt", 1);
    module.add_output("ap_start", 1);
    module.add_input("ap_done", 1);

    // scalar that must be there
    module.add_output("scalar00", 32);
}

fn axi_local_signals(module: &mut v::Module) {
    module.add_decl(v::Decl::new_reg("wstate", 2));
    module.add_decl(v::Decl::new_reg("wnext", 2));
    module.add_decl(v::Decl::new_reg("waddr", 6)); // write address
    module.add_decl(v::Decl::new_wire("wmask", 32)); // valid bits for write data
    module.add_decl(v::Decl::new_wire("aw_hs", 1)); // address write hand shake
    module.add_decl(v::Decl::new_wire("w_hs", 1)); // write hand shake
    module.add_decl(v::Decl::new_reg("rstate", 2)); // = RDRESET; // read fsm state
    module.add_decl(v::Decl::new_reg("rnext", 2)); // next fsm state for read fsm
    module.add_decl(v::Decl::new_reg("rdata", 32)); // read data
    module.add_decl(v::Decl::new_wire("ar_hs", 1)); // address read hand shake
    module.add_decl(v::Decl::new_wire("raddr", 6)); // read address
    module.add_decl(v::Decl::new_reg("int_ap_start", 1)); // = 1'b0;
    module.add_decl(v::Decl::new_reg("int_ap_done", 1)); // = 1'b0;
    module.add_decl(v::Decl::new_reg("int_gie", 1)); // = 1'b0;
    module.add_decl(v::Decl::new_reg("int_ier", 2)); // = 2'b0;
    module.add_decl(v::Decl::new_reg("int_isr", 2)); // = 2'b0;
    module.add_decl(v::Decl::new_reg("int_scalar00", 32)); // = 32'b0;
}

fn axi_write_fsm(module: &mut v::Module) {
    // ready to receive write address when fsm is idle
    // assign AWREADY = (wstate == WRIDLE);
    module.add_stmt(v::Parallel::Assign(
        "AWREADY".into(),
        v::Expr::new_eq("wstate", "WRIDLE"),
    ));
    // ready to receive data when fsm is in WRDATA
    // assign WREADY = (wstate == WRDATA);
    module.add_stmt(v::Parallel::Assign(
        "WREADY".into(),
        v::Expr::new_eq("wstate", "WRDATA"),
    ));
    // assign BRESP = 2'b00;
    module.add_stmt(v::Parallel::Assign(
        "BRESP".into(),
        v::Expr::new_ulit_dec(2, "00"),
    ));
    // assign BVALID = (wstate == WRRESP);
    module.add_stmt(v::Parallel::Assign(
        "BVALID".into(),
        v::Expr::new_eq("wstate", "WRRESP"),
    ));

    // assign wmask = { {8{WSTRB[3]}}, {8{WSTRB[2]}}, {8{WSTRB[1]}}, {8{WSTRB[0]}} };
    let mut expr_concat = v::ExprConcat::default();
    expr_concat
        .add_expr(v::Expr::new_repeat(8, v::Expr::new_index_bit("WSTRB", 3)));
    expr_concat
        .add_expr(v::Expr::new_repeat(8, v::Expr::new_index_bit("WSTRB", 2)));
    expr_concat
        .add_expr(v::Expr::new_repeat(8, v::Expr::new_index_bit("WSTRB", 2)));
    expr_concat
        .add_expr(v::Expr::new_repeat(8, v::Expr::new_index_bit("WSTRB", 0)));
    module.add_stmt(v::Parallel::Assign("wmask".into(), expr_concat.into()));

    // assign aw_hs = AWVALID & AWREADY;
    module.add_stmt(v::Parallel::Assign(
        "aw_hs".into(),
        v::Expr::new_bit_and("AWVALID", "AWREADY"),
    ));

    // assign w_hs = WVALID & WREADY;
    module.add_stmt(v::Parallel::Assign(
        "w_hs".into(),
        v::Expr::new_bit_and("WVALID", "WREADY"),
    ));

    // write fsm transition
    let mut transition = v::ParallelProcess::new_always();
    transition.set_event(v::Sequential::new_posedge("ACLK"));
    let mut ifelse = v::SequentialIfElse::new("ARESET".into());
    ifelse.add_seq(v::Sequential::new_nonblk_assign(
        "wstate".into(),
        "WRRESET".into(),
    ));
    ifelse.set_else(v::Sequential::new_nonblk_assign(
        "wstate".into(),
        "wnext".into(),
    ));
    transition.add_seq(ifelse.into());
    module.add_stmt(transition);

    // wnext
    let mut wnext_always = v::ParallelProcess::new_always();
    wnext_always.set_event(v::Sequential::Wildcard);
    let mut case = v::Case::new("WRRESET".into());

    add_fsm_transition(&mut case, "wnext", "WRIDLE", "WRDATA", "AWVALID");
    add_fsm_transition(&mut case, "wnext", "WRDATA", "WRRESP", "WVALID");
    add_fsm_transition(&mut case, "wnext", "WRRESP", "WRIDLE", "BREADY");

    let mut default = v::CaseDefault::default();
    default.add_seq(v::Sequential::new_blk_assign(
        "wstate".into(),
        "WRIDLE".into(),
    ));
    case.set_default(default);
    wnext_always.add_seq(v::Sequential::new_case(case));
    module.add_stmt(wnext_always);

    // waddr
    let mut waddr_always = v::ParallelProcess::new_always();
    waddr_always.set_event(v::Sequential::new_posedge("ACLK"));
    let mut if_aclk_en = v::SequentialIfElse::new("ACLK_EN".into());
    let mut if_aw_hs = v::SequentialIfElse::new("aw_hs".into());
    if_aw_hs.add_seq(v::Sequential::new_nonblk_assign(
        "waddr".into(),
        v::Expr::new_slice("AWADDR", v::Expr::new_int(5), v::Expr::new_int(0)),
    ));
    if_aclk_en.add_seq(if_aw_hs.into());
    waddr_always.add_seq(if_aclk_en.into());
    module.add_stmt(waddr_always);
}

fn axi_read_fsm(module: &mut v::Module) {
    // ready to receive write address when fsm is idle
    // assign ARREADY = (rstate == RDIDLE);
    module.add_stmt(v::Parallel::Assign(
        "ARREADY".into(),
        v::Expr::new_eq("rstate", "RDIDLE"),
    ));
    // ready to receive data when fsm is in WRDATA
    // assign RDATA = rdata;
    module.add_stmt(v::Parallel::Assign("RDATA".into(), "rdata".into()));
    // assign RRESP = 2'b00;
    module.add_stmt(v::Parallel::Assign(
        "RRESP".into(),
        v::Expr::new_ulit_dec(2, "00"),
    ));
    // assign RVALID = (rstate == RDDATA);
    module.add_stmt(v::Parallel::Assign(
        "RVALID".into(),
        v::Expr::new_eq("wstate", "WRRESP"),
    ));

    // assign ar_hs = ARVALID & ARREADY;
    module.add_stmt(v::Parallel::Assign(
        "ar_hs".into(),
        v::Expr::new_bit_and("ARVALID", "ARREADY"),
    ));

    // assign raddr = ARADDR[5:0];
    module.add_stmt(v::Parallel::Assign(
        "raddr".into(),
        v::Expr::new_slice("ARADDR", v::Expr::new_int(5), v::Expr::new_int(0)),
    ));

    // read fsm transition
    let mut transition = v::ParallelProcess::new_always();
    transition.set_event(v::Sequential::new_posedge("ACLK"));
    let mut ifelse = v::SequentialIfElse::new("ARESET".into());
    ifelse.add_seq(v::Sequential::new_nonblk_assign(
        "rstate".into(),
        "RDRESET".into(),
    ));
    ifelse.set_else(v::Sequential::new_nonblk_assign(
        "wstate".into(),
        "wnext".into(),
    ));
    transition.add_seq(ifelse.into());
    module.add_stmt(transition);

    // wnext
    let mut wnext_always = v::ParallelProcess::new_always();
    wnext_always.set_event(v::Sequential::Wildcard);
    let mut case = v::Case::new("WRRESET".into());

    add_fsm_transition(&mut case, "wnext", "WRIDLE", "WRDATA", "AWVALID");
    add_fsm_transition(&mut case, "wnext", "WRDATA", "WRRESP", "WVALID");
    add_fsm_transition(&mut case, "wnext", "WRRESP", "WRIDLE", "BREADY");

    let mut default = v::CaseDefault::default();
    default.add_seq(v::Sequential::new_blk_assign(
        "wstate".into(),
        "WRIDLE".into(),
    ));
    case.set_default(default);
    wnext_always.add_seq(v::Sequential::new_case(case));
    module.add_stmt(wnext_always);

    // waddr
    let mut waddr_always = v::ParallelProcess::new_always();
    waddr_always.set_event(v::Sequential::new_posedge("ACLK"));
    let mut if_aclk_en = v::SequentialIfElse::new("ACLK_EN".into());
    let mut if_aw_hs = v::SequentialIfElse::new("aw_hs".into());
    if_aw_hs.add_seq(v::Sequential::new_nonblk_assign(
        "waddr".into(),
        v::Expr::new_slice("AWADDR", v::Expr::new_int(5), v::Expr::new_int(0)),
    ));
    if_aclk_en.add_seq(if_aw_hs.into());
    waddr_always.add_seq(if_aclk_en.into());
    module.add_stmt(waddr_always);
}

fn add_fsm_transition(
    case: &mut v::Case,
    next_reg: &str,
    cur_state: &str,
    next_state: &str,
    condition: &str,
) {
    let mut branch = v::CaseBranch::new(v::Expr::new_ref(cur_state));
    let mut ifelse = v::SequentialIfElse::new(v::Expr::new_ref(condition));
    ifelse.add_seq(v::Sequential::new_blk_assign(
        next_reg.into(),
        next_state.into(),
    ));
    ifelse.set_else(v::Sequential::new_blk_assign(
        next_reg.into(),
        cur_state.into(),
    ));
    branch.add_seq(ifelse.into());
    case.add_branch(branch);
}
