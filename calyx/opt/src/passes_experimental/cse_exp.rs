use std::{cell::RefCell, collections::HashMap};

use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir};

const SUPPORTED_STD: &[&str] = &["std_add"];

pub struct CseExp {
    available_expressions: AvailableExpressions,
}

impl Named for CseExp {
    fn name() -> &'static str {
        "cse-exp"
    }

    fn description() -> &'static str {
        "replace common subexpression uses with already computed values when possible"
    }
}

impl Default for CseExp {
    fn default() -> Self {
        CseExp {
            available_expressions: AvailableExpressions {
                current_depth: -1,
                safe_depth: -1,
                running_expressions: HashMap::<
                    i32,
                    HashMap<String, ExpressionMetadata>,
                >::new(),
                per_group_expressions: HashMap::<
                    String,
                    HashMap<String, ExpressionMetadata>,
                >::new(),
            },
        }
    }
}

struct ExpressionMetadata {
    reg_port: ir::RRC<ir::Port>, // id of reg to grab expression from
    group: String,               // group name that created the expression
}

struct AvailableExpressions {
    current_depth: i32,
    safe_depth: i32,
    running_expressions: HashMap<i32, HashMap<String, ExpressionMetadata>>,
    per_group_expressions: HashMap<String, HashMap<String, ExpressionMetadata>>,
}

impl AvailableExpressions {
    // stringifys value of cell prototype
    pub fn get_val(port: &ir::Port) -> String {
        match port.cell_parent().borrow().prototype {
            ir::CellType::Constant { val, .. } => return val.to_string(),
            ir::CellType::Component { name } => return name.to_string(),
            ir::CellType::Primitive { .. } => {
                let port_prefix = port.cell_parent().borrow().name();
                let port_suffix = port.name;
                return format!("{port_prefix}{port_suffix}");
            }
            ir::CellType::ThisComponent => {
                return "absolutely no idea".to_string();
            }
        }
    }
    /*
        increment depth and potentially safe depth, allocating
        a new HashMap for this depth's expressions
    */
    fn inc_depth(&mut self, safe_depth: bool) -> () {
        // invariant check
        assert!(
            self.safe_depth <= self.current_depth,
            "safe depth somehow exceeded current depth"
        );
        // only increment if current_depth is on par with safe depth,
        // otherwise either safe_depth is false OR we are in an unsafe
        // zone.
        if safe_depth && self.current_depth == self.safe_depth {
            self.current_depth += 1;
            self.safe_depth += 1;
        } else {
            self.current_depth += 1;
        }
        // this key should never exist already
        let dbg_depth = self.current_depth;
        if self.running_expressions.contains_key(&self.current_depth) {
            panic!(
                "running expressions somehow already contains current depth {dbg_depth} key"
            );
        }
        let current_depth_expressions: HashMap<String, ExpressionMetadata> =
            HashMap::<String, ExpressionMetadata>::new();
        self.running_expressions
            .insert(self.current_depth, current_depth_expressions);
        log::debug!(
            "incremented current depth to {dbg_depth} and allocated hashmap for expressions"
        );
    }
    /*
        decrement depth and potentially safe depth, deleting the HashMap
        allocated for this depth's expressions
    */
    fn dec_depth(&mut self) -> () {
        // invariant check
        let dbg_deleted_depth = self.current_depth;
        assert!(
            self.safe_depth <= self.current_depth,
            "safe depth somehow exceeded current depth"
        );
        if self.current_depth == self.safe_depth {
            self.safe_depth -= 1;
            self.current_depth -= 1;
        } else {
            self.current_depth -= 1;
        }
        let dbg_depth = self.current_depth;
        self.running_expressions.remove(&self.current_depth);
        log::debug!(
            "decremented current depth to {dbg_depth} and removed hashmap for expressions at depth {dbg_deleted_depth}"
        );
    }

    /*
        add to current_depth's running_expressions available subexpressions from
        supported operations
    */
    fn add_exp(
        &mut self,
        assignments: &Vec<ir::Assignment<ir::Nothing>>, // a specific group's assignments
        group: String, // the group with the assignments in question
    ) -> () {
        let mut intermediate_exp: HashMap<String, String> =
            HashMap::<String, String>::new();
        let mut completed_exp = HashMap::<String, String>::new();
        for assign in assignments.iter() {
            // early breakouts
            if assign.dst.borrow().is_hole() {
                continue;
            }
            let operation =
                match assign.dst.borrow().cell_parent().borrow().type_name() {
                    Some(v) => v,
                    None => continue,
                };
            if !(SUPPORTED_STD.contains(&operation.to_string().as_str())) {
                // here we check if a register is latching an existing subexpression
                let dst_port_name = assign.dst.borrow().name;
                if operation.to_string().as_str() == "std_reg"
                    && dst_port_name.to_string().as_str() == "in"
                {
                    let latching_cadidate =
                        assign.src.borrow().cell_parent().borrow().name();
                    let src_port_name = assign.src.borrow().name;
                    log::debug!(
                        "latching candidate {latching_cadidate}.{src_port_name}"
                    );
                    if completed_exp
                        .contains_key(&latching_cadidate.to_string())
                        && self.current_depth <= self.safe_depth
                        && src_port_name.to_string().as_str() == "out"
                    {
                        let new_exp = ExpressionMetadata {
                            reg_port: assign
                                .dst
                                .borrow()
                                .cell_parent()
                                .borrow()
                                .get("out"), // <------ right now only the in port is written down, we need it to be the .out port. TODO
                            group: group.clone(),
                        };
                        match self
                            .running_expressions
                            .get_mut(&self.current_depth)
                        {
                            Some(current_depth_expressions) => {
                                match completed_exp
                                    .get(&latching_cadidate.to_string())
                                {
                                    Some(string_expression) => {
                                        let dbg_parent = new_exp
                                            .reg_port
                                            .borrow()
                                            .cell_parent()
                                            .borrow()
                                            .name();
                                        let dbg_port =
                                            new_exp.reg_port.borrow().name;
                                        let dbg_depth = self.current_depth;
                                        log::debug!(
                                            "[GEN] adding expression {string_expression} with parent port {dbg_parent}.{dbg_port} to running expressions at depth {dbg_depth}"
                                        );
                                        current_depth_expressions.insert(
                                            string_expression.to_string(),
                                            new_exp,
                                        );
                                    }
                                    None => {
                                        panic!(
                                            "string expression not found in current depth expressions"
                                        );
                                    }
                                }
                            }
                            None => {
                                panic!(
                                    "current depth not found in running expressions"
                                );
                            }
                        }
                    }
                }
                // TODO: ensure expresion is latched and safe_depth is >= cur_depth before adding to avaialble expresions
                continue;
            }
            // check intermediate_exp if already contains expression
            let operation_cell_name =
                assign.dst.borrow().cell_parent().borrow().name();
            if !intermediate_exp.contains_key(&operation_cell_name.to_string())
            {
                intermediate_exp.insert(
                    operation_cell_name.to_string(),
                    AvailableExpressions::get_val(&assign.src.borrow()),
                );
                continue;
            }
            // else we have completed this subexpression
            else {
                let dest =
                    intermediate_exp.get(&operation_cell_name.to_string());
                match dest {
                    Some(destination) => {
                        // grab full subexpression
                        let cdepth = self.current_depth;
                        let source =
                            AvailableExpressions::get_val(&assign.src.borrow());
                        let expression =
                            format!("{source}({operation}){destination}");
                        log::debug!(
                            "added {expression} for depth {cdepth} to completed (intermediate) expressions"
                        );
                        completed_exp.insert(
                            operation_cell_name.to_string(),
                            expression,
                        );
                    }
                    None => {
                        panic!("missing key?");
                    }
                }
            }
        }
    }

    /*
        identify destroyed expressions from register overwrites
        and remove from all depths.
    */
    fn kill_exp(
        &mut self,
        assignments: &Vec<ir::Assignment<ir::Nothing>>,
        group: String,
    ) {
        let mut remove_expressions: Vec<String> = Vec::new();
        for assign in assignments.iter() {
            if assign.dst.borrow().is_hole() {
                continue;
            }
            let operation =
                match assign.dst.borrow().cell_parent().borrow().type_name() {
                    Some(v) => v,
                    None => continue,
                };
            // we need to see if a register that is containing a currently latched
            // subexpression is being overwritted
            let dst_port = assign.dst.borrow();
            if operation.to_string().as_str() == "std_reg"
                && dst_port.name.to_string().as_str() == "in"
            {
                for depth in 0..(self.current_depth + 1) {
                    let e = self.running_expressions.get_mut(&depth);
                    match e {
                        Some(expressions) => {
                            for (string_expression, metadata) in
                                expressions.into_iter()
                            {
                                if metadata.group != group
                                    && metadata
                                        .reg_port
                                        .borrow()
                                        .cell_parent()
                                        .borrow()
                                        .name()
                                        == dst_port
                                            .cell_parent()
                                            .borrow()
                                            .name()
                                {
                                    remove_expressions
                                        .push(string_expression.to_string());
                                }
                            }
                        }
                        None => {
                            panic!("no HashMap allocated for depth {depth}?");
                        }
                    }
                }
            }
            for killed_expression in remove_expressions.iter() {
                for depth in 0..(self.current_depth + 1) {
                    let e = self.running_expressions.get_mut(&depth);
                    match e {
                        Some(expressions) => {
                            if expressions.remove(killed_expression).is_some() {
                                log::debug!(
                                    "[KILL] removed expression {killed_expression} from available expressions at depth {depth}"
                                );
                            }
                        }
                        None => {
                            panic!("no HashMap allocated for depth {depth}?");
                        }
                    }
                }
            }
        }
    }

    /*
        Do one of two things:
            1) if group not in self.group_expressions, do
            self.group_expressions[group] = self.running_expressions
            2) else, do self.group_expressions[group] ∩ self.running_expressions
    */
    fn group_exp(&mut self, group: String) {
        if self.per_group_expressions.contains_key(&group) {
            // do 2)
            let mut remove_expressions: Vec<String> = Vec::new();
            let g_e = self.per_group_expressions.get_mut(&group);
            match g_e {
                Some(group_expressions) => {
                    for (group_string_expression, _) in &mut *group_expressions
                    {
                        let mut remove_flag = true;
                        for depth in 0..(self.current_depth + 1) {
                            let e = self.running_expressions.get_mut(&depth);
                            match e {
                                Some(expressions) => {
                                    if expressions
                                        .contains_key(group_string_expression)
                                    {
                                        remove_flag = false;
                                    }
                                }
                                None => {
                                    panic!(
                                        "no HashMap allocated for depth {depth}?"
                                    );
                                }
                            }
                        }
                        if remove_flag {
                            remove_expressions
                                .push(group_string_expression.clone());
                        }
                    }
                    for removed_expression in remove_expressions.iter() {
                        if group_expressions
                            .remove(removed_expression)
                            .is_some()
                        {
                            log::debug!(
                                "[GROUP-KILL] removed expression {removed_expression} from availalbe expressions for group {group}"
                            );
                        } else {
                            panic!(
                                "expected expression to exist in group expressions"
                            );
                        }
                    }
                }
                None => {
                    panic!("expected group expressions to exist");
                }
            }
        } else {
            // do 1)
            let mut new_group_expressions: HashMap<String, ExpressionMetadata> =
                HashMap::<String, ExpressionMetadata>::new();
            for depth in 0..(self.current_depth + 1) {
                let e = self.running_expressions.get_mut(&depth);
                match e {
                    Some(expressions) => {
                        for (string_expression, metadata) in
                            expressions.into_iter()
                        {
                            new_group_expressions.insert(
                                string_expression.clone(),
                                ExpressionMetadata {
                                    reg_port: metadata.reg_port.clone(),
                                    group: metadata.group.clone(),
                                },
                            );
                        }
                    }
                    None => {
                        panic!("no HashMap allocated for depth {depth}?");
                    }
                }
            }
            let dbg_depth = self.current_depth;
            log::debug!(
                "[GROUP-GEN] inserted all running expressions from depth {dbg_depth} downwards for group {group}"
            );
            self.per_group_expressions
                .insert(group, new_group_expressions);
        }
    }
    /*
        in-place mutate a group given its availalbe expressions by doing
        the following for supported operations:
            1) identify subexpressions created and used within the group
            2) figure out which of those subexpressions have already been
            saved in per_group expressions
            3) replace all "=(redundant_operation).out" with latched register
            outs
    */
    fn optimize(
        &mut self,
        group_obj: &mut std::cell::RefMut<ir::Group>,
        group: String, // the group with the assignments in question
    ) -> () {
        let mut intermediate_exp: HashMap<String, String> =
            HashMap::<String, String>::new();
        let mut completed_exp = HashMap::<String, String>::new();
        let mut cse_rewriter: ir::rewriter::PortRewriteMap =
            ir::rewriter::PortRewriteMap::new();
        let assignments = &mut group_obj.assignments;
        for assign in &mut assignments.iter_mut() {
            // early breakouts
            if assign.dst.borrow().is_hole() {
                continue;
            }
            let operation =
                match assign.dst.borrow().cell_parent().borrow().type_name() {
                    Some(v) => v,
                    None => continue,
                };
            if !(SUPPORTED_STD.contains(&operation.to_string().as_str())) {
                // here we check if an operation is reading from a redundant operation
                let cse_candidate_operation = match assign
                    .src
                    .borrow()
                    .cell_parent()
                    .borrow()
                    .type_name()
                {
                    Some(v) => v,
                    None => continue,
                };
                if !(SUPPORTED_STD
                    .contains(&cse_candidate_operation.to_string().as_str()))
                {
                    continue;
                }
                if assign.src.borrow().name != "out" {
                    continue;
                }
                // at this point we are confident that its a supported operation and a .out port
                // being read by some other cell. check if it contains a subexpression thats already computed
                let supported_operation_key = assign
                    .src
                    .borrow()
                    .cell_parent()
                    .borrow()
                    .name()
                    .to_string();
                let string_expression =
                    match completed_exp.get(&supported_operation_key) {
                        Some(v) => v,
                        None => continue,
                    };
                let current_group_subexpressions = match self
                    .per_group_expressions
                    .get(&group)
                {
                    Some(v) => v,
                    None => {
                        panic!(
                            "group should have available expressions at this point"
                        )
                    }
                };
                match current_group_subexpressions.get(string_expression) {
                    Some(potential_common_subexpression) => {
                        if potential_common_subexpression.group != group {
                            log::debug!(
                                "common subexpression {string_expression} identified in {group}"
                            );
                            /*
                                i think now you add a mapping from redun_calculation.out to latched_exp_reg.out
                                aka mapping from assignment src to cse port
                            */
                            let dbg_canonical_src =
                                assign.src.borrow().canonical();
                            let dbg_canonical_val =
                                potential_common_subexpression
                                    .reg_port
                                    .clone()
                                    .borrow()
                                    .canonical();
                            log::debug!(
                                "[OPTIMIZE] applying mapping from {dbg_canonical_src} -> {dbg_canonical_val} for group {group}"
                            );
                            cse_rewriter.insert(
                                assign.src.clone().borrow().canonical(),
                                potential_common_subexpression.reg_port.clone(),
                            );
                            let rewriter = ir::Rewriter {
                                port_map: cse_rewriter.clone(),
                                ..Default::default()
                            };
                            let mut asgn = assign.clone();
                            rewriter.rewrite_assign(&mut asgn);
                            *assign = asgn;
                        }
                    }
                    None => continue,
                }
                continue;
            }
            // check intermediate_exp if already contains expression
            let operation_cell_name =
                assign.dst.borrow().cell_parent().borrow().name();
            if !intermediate_exp.contains_key(&operation_cell_name.to_string())
            {
                intermediate_exp.insert(
                    operation_cell_name.to_string(),
                    AvailableExpressions::get_val(&assign.src.borrow()),
                );
                continue;
            }
            // else we have completed this subexpression
            else {
                let dest =
                    intermediate_exp.get(&operation_cell_name.to_string());
                match dest {
                    Some(destination) => {
                        // grab full subexpression
                        let cdepth = self.current_depth;
                        let source =
                            AvailableExpressions::get_val(&assign.src.borrow());
                        let expression =
                            format!("{source}({operation}){destination}");
                        log::debug!(
                            "added {expression} for depth {cdepth} to completed (intermediate) expressions"
                        );
                        completed_exp.insert(
                            operation_cell_name.to_string(),
                            expression,
                        );
                    }
                    None => {
                        panic!("missing key?");
                    }
                }
            }
        }
        // let rewriter = ir::Rewriter {
        //     port_map: cse_rewriter,
        //     ..Default::default()
        // };
        // log::debug!("rewriter invoked for group {group}");
        // let mut asgns = group_obj.assignments.clone();
        // for assign in asgns.iter_mut() {
        //     rewriter.rewrite_assign(assign);
        // }
        // group_obj.assignments = asgns;
        // rewriter.rewrite_control(&mut comp.control.borrow_mut());
    }
}

impl Visitor for CseExp {
    /*
        Start is treated like a seq block, so simple safe increment
        of depth
    */
    fn start(
        &mut self,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        log::debug!("toplevel start");
        // create depth 0 dictionary, this is basically a seq block
        self.available_expressions.inc_depth(true);
        Ok(Action::Continue)
    }
    fn start_seq(
        &mut self,
        _s: &mut calyx_ir::Seq,
        _comp: &mut calyx_ir::Component,
        _sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        log::debug!("start_seq");
        self.available_expressions.inc_depth(true);
        Ok(Action::Continue)
    }
    fn finish_seq(
        &mut self,
        _s: &mut calyx_ir::Seq,
        _comp: &mut calyx_ir::Component,
        _sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        log::debug!("finish_seq");
        self.available_expressions.dec_depth();
        Ok(Action::Continue)
    }
    /*
        Do:
            1) add expressions this group creates
            2) remove expressions this group killed
            3) update the expressions availalbe to this group specifically
               which is either...
                3.0) adding the current running expressions entirely if
                     there arent expressions logged for the group already
                3.1) adding the intersection of current running expressions
                     /w this groups logged expressions
    */
    fn enable(
        &mut self,
        _s: &mut calyx_ir::Enable,
        _comp: &mut calyx_ir::Component,
        _sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        let group = &_s.group;
        let group_name = group.borrow().name().to_string();
        log::debug!("group {group_name} enable");
        self.available_expressions
            .add_exp(&group.borrow().assignments, group_name.clone());
        self.available_expressions
            .kill_exp(&group.borrow().assignments, group_name.clone());
        self.available_expressions.group_exp(group_name.clone());
        Ok(Action::Continue)
    }

    /*
        Remove the identified redundant common subexpressions in each group
    */
    fn finish(
        &mut self,
        _comp: &mut calyx_ir::Component,
        _sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        log::debug!("optimize");
        for group in _comp.get_groups_mut().iter_mut() {
            let group_name = group.borrow().name().to_string();
            log::debug!("group {group_name}");
            self.available_expressions
                .optimize(&mut group.borrow_mut(), group_name);
        }
        Ok(Action::Continue)
    }
}
