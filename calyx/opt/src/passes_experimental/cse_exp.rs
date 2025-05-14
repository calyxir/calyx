use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir};
use ir::Control::{self as Control};
use std::{collections::HashMap, rc::Rc};

const SUPPORTED_STD: &[&str] = &["std_add"];

pub struct CseExp {
    available_expressions: AvailableExpressions,
}

impl Named for CseExp {
    fn name() -> &'static str {
        "cse-exp"
    }

    fn description() -> &'static str {
        "constant focused common subexpression elimination"
    }
}

impl Default for CseExp {
    fn default() -> Self {
        CseExp {
            available_expressions: AvailableExpressions {
                enable_addition: true,
                running_expressions:
                    HashMap::<String, Vec<ExpressionMetadata>>::new(),
                per_group_expressions: HashMap::<
                    String,
                    HashMap<String, Vec<ExpressionMetadata>>,
                >::new(),
            },
        }
    }
}

struct ExpressionMetadata {
    reg_port: ir::RRC<ir::Port>, // id of reg to grab expression from
    group: String,               // group name that created the expression
}

impl Clone for ExpressionMetadata {
    fn clone(&self) -> Self {
        ExpressionMetadata {
            reg_port: self.reg_port.clone(),
            group: self.group.clone(),
        }
    }
}

struct AvailableExpressions {
    enable_addition: bool, // true enables full add_exp, kill_exp, group_exp, false only allows kill_exp
    running_expressions: HashMap<String, Vec<ExpressionMetadata>>, // its a vector to deal with duplicates
    per_group_expressions:
        HashMap<String, HashMap<String, Vec<ExpressionMetadata>>>,
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

    // unions two available expression structs
    pub fn union(
        one: &AvailableExpressions,
        two: &AvailableExpressions,
    ) -> (
        HashMap<String, Vec<ExpressionMetadata>>,
        HashMap<String, HashMap<String, Vec<ExpressionMetadata>>>,
    ) {
        // we follow one's order of stuff for the union, i dont know of anything better
        // two's additions will be after one's if they share the same value
        let mut running_union = one.running_expressions.clone();
        for (string_expression, metadata_vec) in two.running_expressions.iter()
        {
            if !one.running_expressions.contains_key(string_expression) {
                running_union
                    .insert(string_expression.clone(), metadata_vec.clone());
            } else {
                for metadata in metadata_vec {
                    if !one
                        .running_contains_metadata(string_expression, metadata)
                    {
                        // this is a new metadata in a shared string_expression
                        let mut new_metadata_vec = running_union
                            .get(string_expression)
                            .expect("should contain metadata_vec")
                            .clone();
                        new_metadata_vec.push(metadata.clone());
                        running_union.insert(
                            string_expression.clone(),
                            new_metadata_vec,
                        );
                    }
                }
            }
        }
        // this is still a group intersection
        let group_intersection =
            AvailableExpressions::group_intersection(one, two);
        return (running_union, group_intersection);
    }

    // intersects two available expression structs' per_group expressions
    pub fn group_intersection(
        one: &AvailableExpressions,
        two: &AvailableExpressions,
    ) -> HashMap<String, HashMap<String, Vec<ExpressionMetadata>>> {
        let mut group_intersection =
            HashMap::<String, HashMap<String, Vec<ExpressionMetadata>>>::new();
        for (group, expressions) in one.per_group_expressions.iter() {
            // do intersection if both available expressions have the group
            if two.per_group_expressions.contains_key(group) {
                for (string_expression, metadata_vec) in expressions {
                    for metadata in metadata_vec {
                        if two.group_contains_metadata(
                            &group,
                            &string_expression,
                            &metadata,
                        ) {
                            if !group_intersection.contains_key(group) {
                                let group_expressions = HashMap::<
                                    String,
                                    Vec<ExpressionMetadata>,
                                >::new(
                                );
                                group_intersection
                                    .insert(group.clone(), group_expressions);
                            }
                            let group_expressions =
                                group_intersection.get_mut(group).expect(
                                    &format!("expected expressions {group}"),
                                );
                            if group_expressions.contains_key(string_expression)
                            {
                                let mut group_metadata_vec = group_expressions.get(string_expression).expect(&format!("expected metadata vec {string_expression}")).clone();
                                group_metadata_vec.push(metadata.clone());
                                group_expressions
                                    .insert(group.clone(), group_metadata_vec);
                            } else {
                                let mut group_metadata_vec =
                                    Vec::<ExpressionMetadata>::new();
                                group_metadata_vec.push(metadata.clone());
                                group_expressions
                                    .insert(group.clone(), group_metadata_vec);
                            }
                        }
                    }
                }
            } else {
                // else do union since the group was unique to the one branch
                group_intersection.insert(group.clone(), expressions.clone());
            }
        }
        // catch groups that two saw that one didn't see
        for (group, expressions) in two.per_group_expressions.iter() {
            if !one.per_group_expressions.contains_key(group) {
                group_intersection.insert(group.clone(), expressions.clone());
            }
        }
        return group_intersection;
    }

    // intersects two availalbe expression structs
    pub fn intersection(
        one: &AvailableExpressions,
        two: &AvailableExpressions,
    ) -> (
        HashMap<String, Vec<ExpressionMetadata>>,
        HashMap<String, HashMap<String, Vec<ExpressionMetadata>>>,
    ) {
        // we follow one's order of stuff for the intersection, since it has to exist in both branches its an alright order to follow
        let mut running_intersection =
            HashMap::<String, Vec<ExpressionMetadata>>::new();
        for (string_expression, metadata_vec) in one.running_expressions.iter()
        {
            let mut new_metadata_vec = Vec::<ExpressionMetadata>::new();
            for metadata in metadata_vec {
                if two.running_contains_metadata(&string_expression, &metadata)
                {
                    new_metadata_vec.push(metadata.clone());
                }
            }
            if new_metadata_vec.len() > 0 {
                running_intersection
                    .insert(string_expression.clone(), new_metadata_vec);
            }
        }
        let group_intersection =
            AvailableExpressions::group_intersection(one, two);
        return (running_intersection, group_intersection);
    }
    fn clone(&self) -> AvailableExpressions {
        AvailableExpressions {
            enable_addition: self.enable_addition,
            running_expressions: self.running_expressions.clone(),
            per_group_expressions: self.per_group_expressions.clone(),
        }
    }
    // for checking running_expressions
    fn running_contains_metadata(
        &self,
        string_expression: &String,
        metadata: &ExpressionMetadata,
    ) -> bool {
        let mut contains_flag = false;
        match self.running_expressions.get(string_expression) {
            Some(metadata_vec) => {
                for met in metadata_vec {
                    if met.group == metadata.group
                        && met.reg_port == metadata.reg_port
                    {
                        contains_flag = true;
                    }
                }
            }
            None => {}
        }
        return contains_flag;
    }

    // for checking group_expressions
    fn group_contains_metadata(
        &self,
        group: &String,
        string_expression: &String,
        metadata: &ExpressionMetadata,
    ) -> bool {
        let mut contains_flag = false;
        match self.per_group_expressions.get(group) {
            Some(expressions) => match expressions.get(string_expression) {
                Some(metadata_vec) => {
                    for met in metadata_vec {
                        if met.group == metadata.group
                            && met.reg_port == metadata.reg_port
                        {
                            contains_flag = true;
                        }
                    }
                }
                None => {}
            },
            None => {
                log::debug!(
                    "function called without containing hashmap for group {group}"
                );
            }
        }
        return contains_flag;
    }

    /*
        given a list of assignments within a group, add to running_expressions
        the corresponding available subexpressions in said group
        (restricted to supported operations)
    */
    fn add_exp(
        &mut self,
        assignments: &Vec<ir::Assignment<ir::Nothing>>, // a specific group's assignments
        group: String, // the group with the assignments in question
    ) -> () {
        if !self.enable_addition {
            log::debug!("enable addition is false, not running add_exp");
            return;
        }
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
                // (since this isn't a supported operation to track common subexpression elimination)
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
                        && src_port_name.to_string().as_str() == "out"
                    {
                        let string_expression = completed_exp
                            .get(&latching_cadidate.to_string()).expect("expected completed expressions to contain latching candidate string");
                        let new_exp = ExpressionMetadata {
                            reg_port: assign
                                .dst
                                .borrow()
                                .cell_parent()
                                .borrow()
                                .get("out")
                                .clone(),
                            group: group.clone(),
                        };
                        let dbg_parent = new_exp
                            .reg_port
                            .borrow()
                            .cell_parent()
                            .borrow()
                            .name();
                        let dbg_port = new_exp.reg_port.borrow().name;
                        match self
                            .running_expressions
                            .get_mut(string_expression)
                        {
                            Some(metadata_vec) => {
                                // existing list of subexpressions will have another source appended on it
                                log::debug!(
                                    "[GEN] adding {string_expression} with parent port {dbg_parent}.{dbg_port} to existing list"
                                );
                                metadata_vec.push(new_exp);
                            }
                            None => {
                                // new list of subexpressions will be allocated
                                let new_metadata_vec = vec![new_exp];
                                log::debug!(
                                    "[GEN] adding expression {string_expression} with parent port {dbg_parent}.{dbg_port} to new list of running expressions"
                                );
                                self.running_expressions.insert(
                                    string_expression.to_string(),
                                    new_metadata_vec,
                                );
                            }
                        }
                    }
                }
            } else {
                // this is a supported operation to track common subexpression elimination

                // check intermediate_exp if already contains expression
                let operation_cell_name =
                    assign.dst.borrow().cell_parent().borrow().name();
                if !intermediate_exp
                    .contains_key(&operation_cell_name.to_string())
                {
                    intermediate_exp.insert(
                        operation_cell_name.to_string(),
                        AvailableExpressions::get_val(&assign.src.borrow()),
                    );
                }
                // else we have completed this subexpression
                else {
                    let destination = intermediate_exp
                        .get(&operation_cell_name.to_string())
                        .expect("missing intermediate expression key");
                    // grab full subexpression
                    let source =
                        AvailableExpressions::get_val(&assign.src.borrow());
                    let expression =
                        format!("{source}({operation}){destination}");
                    log::debug!(
                        "added {expression} to completed (intermediate) expressions"
                    );
                    completed_exp
                        .insert(operation_cell_name.to_string(), expression);
                }
            }
        }
    }

    /*
        identify destroyed expressions from register overwrites
        and remove from running_expressions
    */
    fn kill_exp(
        &mut self,
        assignments: &Vec<ir::Assignment<ir::Nothing>>,
        group: String,
    ) {
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

            // we need to see if a register that is containing a currently latched
            // subexpression is being overwritted
            let dst_port = assign.dst.borrow();
            if operation.to_string().as_str() == "std_reg"
                && dst_port.name.to_string().as_str() == "in"
            {
                let mut updates =
                    HashMap::<String, Vec<ExpressionMetadata>>::new();
                for (string_expression, metadata_vec) in
                    self.running_expressions.iter()
                {
                    let mut new_expression_sources =
                        Vec::<ExpressionMetadata>::new();
                    for metadata in metadata_vec {
                        // either this was introduced in this group, or we don't share a cell_parent
                        // if the above is true then we keep the expression
                        if metadata.group == group
                            || metadata
                                .reg_port
                                .borrow()
                                .cell_parent()
                                .borrow()
                                .name()
                                != dst_port.cell_parent().borrow().name()
                        {
                            new_expression_sources.push(ExpressionMetadata {
                                reg_port: metadata.reg_port.clone(),
                                group: metadata.group.clone(),
                            });
                        } else {
                            let dbg_parent =
                                dst_port.cell_parent().borrow().name();
                            let dbg_port = metadata.reg_port.borrow().name;
                            log::debug!(
                                "[KILL] removed {string_expression} with parent port {dbg_parent}.{dbg_port} from expressions"
                            );
                        }
                    }
                    updates.insert(
                        string_expression.clone(),
                        new_expression_sources,
                    );
                }
                for (key, value) in updates {
                    self.running_expressions.insert(key, value);
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
        if !self.enable_addition {
            log::debug!("enable_addition is false, not running group_exp()");
            return;
        }
        if !self.per_group_expressions.contains_key(&group) {
            // do 1)
            let mut new_group_expressions: HashMap<
                String,
                Vec<ExpressionMetadata>,
            > = HashMap::<String, Vec<ExpressionMetadata>>::new();
            for (string_expression, metadata_vec) in
                self.running_expressions.iter()
            {
                for metadata in metadata_vec.into_iter() {
                    match new_group_expressions.get_mut(string_expression) {
                        Some(group_expression_vec) => group_expression_vec
                            .push(ExpressionMetadata {
                                reg_port: metadata.reg_port.clone(),
                                group: metadata.group.clone(),
                            }),
                        None => {
                            let mut new_group_expression_vec =
                                Vec::<ExpressionMetadata>::new();
                            new_group_expression_vec.push(ExpressionMetadata {
                                reg_port: metadata.reg_port.clone(),
                                group: metadata.group.clone(),
                            });
                            new_group_expressions.insert(
                                string_expression.clone(),
                                new_group_expression_vec,
                            );
                        }
                    }
                }
            }
            log::debug!(
                "[GROUP-GEN] inserted all running expressions for group {group}"
            );
            self.per_group_expressions
                .insert(group, new_group_expressions);
        } else {
            // do 2)
            let mut new_group_expressions =
                HashMap::<String, Vec<ExpressionMetadata>>::new();

            let group_expressions = self
                .per_group_expressions
                .get(&group)
                .expect(&format!("expected {group} expressions to exist"));
            for (group_string_expression, metadata_vec) in group_expressions {
                let mut new_group_expression_vec =
                    Vec::<ExpressionMetadata>::new();
                for metadata in metadata_vec.into_iter() {
                    if self.running_contains_metadata(
                        group_string_expression,
                        metadata,
                    ) {
                        new_group_expression_vec.push(ExpressionMetadata {
                            group: metadata.group.clone(),
                            reg_port: metadata.reg_port.clone(),
                        })
                    } else {
                        let dbg_parent = metadata
                            .reg_port
                            .borrow()
                            .cell_parent()
                            .borrow()
                            .name();
                        let dbg_port = metadata.reg_port.borrow().name;
                        log::debug!(
                            "[GROUP-KILL] removed expression {group_string_expression} with parent port {dbg_parent}.{dbg_port}"
                        );
                    }
                }
                new_group_expressions.insert(
                    group_string_expression.clone(),
                    new_group_expression_vec,
                );
            }
            self.per_group_expressions
                .insert(group.clone(), new_group_expressions);
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
        group: String,
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
                let current_group_subexpressions = self
                    .per_group_expressions
                    .get(&group).expect(&format!("{group} should have available expressions at this point"));
                match current_group_subexpressions.get(string_expression) {
                    Some(potential_common_subexpression_vec) => {
                        if potential_common_subexpression_vec.len() > 0 {
                            // getting the 0th index will get the earliest detected common subexpression
                            let potential_common_subexpression =
                                potential_common_subexpression_vec
                                    .get(0)
                                    .expect(&format!(
                                        "expected zero index expression"
                                    ));
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
                                    potential_common_subexpression
                                        .reg_port
                                        .clone(),
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
                    }
                    None => continue,
                }
            } else {
                // this is a supported operation to track common subexpression elimination

                // check intermediate_exp if already contains expression
                let operation_cell_name =
                    assign.dst.borrow().cell_parent().borrow().name();
                if !intermediate_exp
                    .contains_key(&operation_cell_name.to_string())
                {
                    intermediate_exp.insert(
                        operation_cell_name.to_string(),
                        AvailableExpressions::get_val(&assign.src.borrow()),
                    );
                    continue;
                }
                // else we have completed this subexpression
                else {
                    let destination = intermediate_exp
                        .get(&operation_cell_name.to_string())
                        .expect("missing intermediate expression key");
                    let source =
                        AvailableExpressions::get_val(&assign.src.borrow());
                    let expression =
                        format!("{source}({operation}){destination}");
                    log::debug!(
                        "added {expression} to completed (intermediate) expressions"
                    );
                    completed_exp
                        .insert(operation_cell_name.to_string(), expression);
                }
            }
        }
    }
}

/*
    modified for available expression detection purposes.
*/
trait ExpressionVisitor {
    /// Executed before visiting the children of a [ir::Seq] node.
    fn start_seq(&mut self, _s: &mut ir::Seq) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed after visiting the children of a [ir::Seq] node.
    fn finish_seq(&mut self, _s: &mut ir::Seq) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed before visiting the children of a [ir::Par] node.
    fn start_par(&mut self, _s: &mut ir::Par) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed after visiting the children of a [ir::Par] node.
    fn finish_par(&mut self, _s: &mut ir::Par) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed before visiting the children of a [ir::If] node.
    fn start_if(&mut self, _s: &mut ir::If) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed after visiting the children of a [ir::If] node.
    fn finish_if(&mut self, _s: &mut ir::If) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed before visiting the children of a [ir::While] node.
    fn start_while(&mut self, _s: &mut ir::While) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed after visiting the children of a [ir::While] node.
    fn finish_while(&mut self, _s: &mut ir::While) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed before visiting the children of a [ir::Repeat] node.
    fn start_repeat(&mut self, _s: &mut ir::Repeat) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed after visiting the children of a [ir::Repeat] node.
    fn finish_repeat(&mut self, _s: &mut ir::Repeat) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed at an [ir::Enable] node.
    fn enable(&mut self, _s: &mut ir::Enable) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed at an [ir::Empty] node.
    fn empty(&mut self, _s: &mut ir::Empty) -> VisResult {
        Ok(Action::Continue)
    }

    /// Executed at an [ir::Invoke] node.
    fn invoke(&mut self, _s: &mut ir::Invoke) -> VisResult {
        Ok(Action::Continue)
    }
}

// grabbing all of the private
impl Action {
    /// Run the traversal specified by `next` if this traversal succeeds.
    /// If the result of this traversal is not `Action::Continue`, do not
    /// run `next()`.
    fn and_then_local<F>(self, mut next: F) -> VisResult
    where
        F: FnMut() -> VisResult,
    {
        match self {
            Action::Continue => next(),
            Action::Change(_)
            | Action::Stop
            | Action::SkipChildren
            | Action::StaticChange(_) => Ok(self),
        }
    }
    /// Applies the Change action if `self` is a Change action.
    /// Otherwise passes the action through unchanged
    fn apply_change_local(self, con: &mut Control) -> Action {
        match self {
            Action::Change(c) => {
                *con = *c;
                Action::Continue
            }
            action => action,
        }
    }
    /// Changes a Action::SkipChildren to Action::Continue.
    /// Should be called to indicate the boundary of traversing the children
    /// of a node.
    fn pop_local(self) -> Self {
        match self {
            Action::SkipChildren => Action::Continue,
            x => x,
        }
    }
}

trait ExpressionVisitable {
    /// Perform the traversal.
    fn visit(&mut self, visitor: &mut dyn ExpressionVisitor) -> VisResult;
}

impl ExpressionVisitable for Control {
    fn visit(&mut self, visitor: &mut dyn ExpressionVisitor) -> VisResult {
        let res = match self {
            Control::Seq(ctrl) => visitor
                .start_seq(ctrl)?
                .and_then_local(|| ctrl.stmts.visit(visitor))?
                .pop_local()
                .and_then_local(|| visitor.finish_seq(ctrl))?,
            Control::Par(ctrl) => visitor
                .start_par(ctrl)?
                .and_then_local(|| ctrl.stmts.visit(visitor))?
                .pop_local()
                .and_then_local(|| visitor.finish_par(ctrl))?,
            Control::If(ctrl) => visitor
                .start_if(ctrl)?
                .and_then_local(|| ctrl.tbranch.visit(visitor))?
                .and_then_local(|| ctrl.fbranch.visit(visitor))?
                .pop_local()
                .and_then_local(|| visitor.finish_if(ctrl))?,
            Control::While(ctrl) => visitor
                .start_while(ctrl)?
                .and_then_local(|| ctrl.body.visit(visitor))?
                .pop_local()
                .and_then_local(|| visitor.finish_while(ctrl))?,
            Control::Repeat(ctrl) => visitor
                .start_repeat(ctrl)?
                .and_then_local(|| ctrl.body.visit(visitor))?
                .pop_local()
                .and_then_local(|| visitor.finish_repeat(ctrl))?,
            Control::Enable(ctrl) => visitor.enable(ctrl)?,
            Control::Static(_) => panic!("not supported yet"),
            Control::Empty(ctrl) => visitor.empty(ctrl)?,
            Control::Invoke(data) => visitor.invoke(data)?,
        };
        Ok(res.apply_change_local(self))
    }
}

/// Blanket implementation for Vectors of Visitables
impl<V: ExpressionVisitable> ExpressionVisitable for Vec<V> {
    fn visit(&mut self, visitor: &mut dyn ExpressionVisitor) -> VisResult {
        for t in self {
            let res = t.visit(visitor)?;
            match res {
                Action::Continue
                | Action::SkipChildren
                | Action::Change(_)
                | Action::StaticChange(_) => {
                    continue;
                }
                Action::Stop => return Ok(Action::Stop),
            };
        }
        Ok(Action::Continue)
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
        log::debug!("[START] Toplevel AvailableExpression Analysis");
        // Create a clone of the reference to the Control
        // program.
        let control_ref = Rc::clone(&_comp.control);
        if let Control::Empty(_) = &*control_ref.borrow() {
            // Don't traverse if the control program is empty.
            return Ok(Action::Continue);
        }
        // Mutably borrow the control program and traverse.
        control_ref.borrow_mut().visit(self)?;
        // can't call skip-children here unfortunately since visitor missing pop() call
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
        log::debug!("[FINISH] Optimize");
        for group in _comp.get_groups_mut().iter_mut() {
            let group_name = group.borrow().name().to_string();
            log::debug!("Group: {group_name}");
            self.available_expressions
                .optimize(&mut group.borrow_mut(), group_name);
        }
        Ok(Action::Continue)
    }
}

impl ExpressionVisitor for CseExp {
    fn start_if(&mut self, _s: &mut calyx_ir::If) -> VisResult {
        log::debug!("start_if");
        // need to run both branches separately and combine common outputs.
        let mut true_cse_exp = CseExp {
            available_expressions: self.available_expressions.clone(),
        };
        let mut false_cse_exp = CseExp {
            available_expressions: self.available_expressions.clone(),
        };
        log::debug!("[START] starting true branch");
        let _ = _s.tbranch.visit(&mut true_cse_exp);
        log::debug!("[START] starting false branch");
        let _ = _s.fbranch.visit(&mut false_cse_exp);
        log::debug!("intersecting branches");
        let (intersection_running, intersection_group) =
            AvailableExpressions::intersection(
                &true_cse_exp.available_expressions,
                &false_cse_exp.available_expressions,
            );
        // finally overwrite the current available expressions
        log::debug!("overwriting local expressions with branch intersection");
        self.available_expressions.running_expressions = intersection_running;
        self.available_expressions.per_group_expressions = intersection_group;
        Ok(Action::SkipChildren)
    }
    fn start_par(&mut self, _s: &mut calyx_ir::Par) -> VisResult {
        log::debug!("start_par");
        // first disable enable_addition and save state
        self.available_expressions.enable_addition = false;
        let initial_save_state = self.available_expressions.clone();
        // need to run all branches independently and merge their outputs
        for control in _s.stmts.iter_mut() {
            let mut child_control_cse_exp = CseExp {
                available_expressions: initial_save_state.clone(),
            };
            log::debug!(
                "[START] starting par control child for baseline construction"
            );
            let _ = control.visit(&mut child_control_cse_exp);
            log::debug!(
                "intersection between parent available expression and child control"
            );
            let (intersection_running, intersection_group) =
                AvailableExpressions::intersection(
                    &self.available_expressions,
                    &child_control_cse_exp.available_expressions,
                );
            log::debug!(
                "overwriting local expressions with child intersection"
            );
            self.available_expressions.running_expressions =
                intersection_running;
            self.available_expressions.per_group_expressions =
                intersection_group;
        }
        // at this point all expressions that would have been killed at any point by a child
        // have been killed, and this is reflected in self.available expressions
        self.available_expressions.enable_addition = true;
        let true_baseline_save_state = self.available_expressions.clone();
        for control in _s.stmts.iter_mut() {
            let mut child_control_cse_exp = CseExp {
                available_expressions: true_baseline_save_state.clone(),
            };
            log::debug!("[START] starting par control child for union");
            let _ = control.visit(&mut child_control_cse_exp);
            log::debug!(
                "union between parent available expression and child control"
            );
            let (union_running, union_group) = AvailableExpressions::union(
                &self.available_expressions,
                &child_control_cse_exp.available_expressions,
            );
            log::debug!(
                "overwriting local expressions with child intersection"
            );
            self.available_expressions.running_expressions = union_running;
            self.available_expressions.per_group_expressions = union_group;
        }
        Ok(Action::SkipChildren)
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
    fn enable(&mut self, _s: &mut calyx_ir::Enable) -> VisResult {
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
}
