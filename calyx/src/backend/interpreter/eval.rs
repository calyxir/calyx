use super::inputs::{Inputs, Outputs};
use crate::backend::interpreter::state::State;
use crate::backend::traits::Backend;
use crate::errors::Error;
use crate::lang::ast;
use crate::lang::component::Component;
use crate::lang::context::Context;
use crate::lang::structure::StructureGraph;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

pub trait EvalGraph {
    /// Splits sequential primitive component nodes into two separate nodes.
    /// Splitting sequential primitives into two nodes should turn all
    /// valid structure into a DAG for further analysis. One node will only have input wires,
    /// and the corresponding node will only have output wires.
    ///
    /// Instance map will tentatively point only to the input node.
    ///
    /// Perhaps should switch to using an iterator?
    /// std::mem::replace is used according to a response to this stack overflow post:
    /// https://stackoverflow.com/questions/35936995/mutating-one-field-while-iterating-over-another-immutable-field
    fn split_seq_prims(&mut self);

    /// Helper function for split_seq_prims
    /// Splits a given node into two nodes, one that has all incoming edges
    /// and one that has all outgoing edges
    fn split_node(&mut self, idx: NodeIndex);

    /// Set values for inputs
    fn drive_inputs(&mut self, inputs: &HashMap<ast::Id, Option<i64>>);

    /// Set values for outputs of a component
    fn drive_outputs(
        &mut self,
        idx: &NodeIndex,
        outputs: &HashMap<ast::Id, Option<i64>>,
    );

    /// Helper function for setting the value of the outputs of a state port
    fn drive_state(&mut self, state: &State);

    /// Helper function for setting values of ports
    /// Drives a specific port of a node with a provided value
    /// TODO make documentation clearer
    fn drive_port(
        &mut self,
        idx: &NodeIndex,
        port: String,
        value: &Option<i64>,
    );

    /// Returns all node indices in topological sort order
    fn toposort(&self) -> Result<Vec<NodeIndex>, Error>;

    /// Returns a map of input ports to values
    fn input_values(&self, idx: &NodeIndex) -> HashMap<ast::Id, Option<i64>>;

    /// Simulates every component once in topological sort order
    fn update(
        &mut self,
        interpret: &Interpreter,
        st: &State,
        enabled: Vec<ast::Id>,
    ) -> Result<State, Error>;

    /// Called after update to get back the outputs from the component
    fn read_outputs(&mut self) -> HashMap<ast::Id, Option<i64>>;
}

pub struct Interpreter {
    pub context: Context,
}

impl Backend for Interpreter {
    fn name() -> &'static str {
        "interpreter"
    }

    fn validate(_ctx: &Context) -> Result<(), Error> {
        Ok(())
    }

    fn emit<W: Write>(_ctx: &Context, _file: W) -> Result<(), Error> {
        //super::repl::repl(ctx)
        Ok(())
    }
}

impl Interpreter {
    /// Constructs a new interpreter object from a context
    pub fn new(context: &Context) -> Self {
        Interpreter {
            context: context.clone(),
        }
    }

    /// Runs an individual component with input from a JSON file.
    /// Used for testing.
    pub fn run<W: Write>(
        &self,
        file: &mut W,
        path: PathBuf,
    ) -> Result<(), Error> {
        // Read input JSON file
        let inputs = Inputs::from_file(path)?;
        let comp_name = inputs.component();

        if self.context.is_lib(&comp_name) {
            // Running primitive component
            let prim = self.context.instantiate_primitive(
                &comp_name,
                &comp_name,
                inputs.params().as_slice(),
            )?;
            self.context.insert_component(prim);
        }

        let comp = self.context.get_component(&comp_name)?;

        let st = State::from_component(&comp, &self.context)?;

        let (outputs, st_1) = self.eval(st, &comp_name, inputs.inputs())?;
        if let Ok(json_out) = serde_json::to_string(&Outputs::from(&outputs)) {
            Ok(write!(file, "{}", json_out)?)
        } else {
            // TODO fix this error message to be accurate
            Err(Error::InvalidInputJSON)
        }
    }

    /// Evaluates a component
    /// # Arguments
    ///   * `c` - is the context for the file
    ///   * `st` - is the default starting state of the component
    ///   * `comp_name` - the name of the type of component to run
    ///   * `inputs` - is a map of input port names to values for passing
    ///                inputs to the component during evaluation
    /// # Returns
    ///   Returns a map of output port names to values
    pub fn eval(
        &self,
        st: State,
        comp_name: &ast::Id,
        inputs: HashMap<ast::Id, Option<i64>>,
    ) -> Result<(HashMap<ast::Id, Option<i64>>, State), Error> {
        if self.context.is_lib(comp_name) {
            // Handle library components in a special case
            return self.eval_lib(st, comp_name, inputs);
        }

        // User-defined components
        let comp = self.context.get_component(comp_name)?;
        //  structure graph
        let st_1 = self.eval_c(&inputs, &comp.control, &comp.structure, &st);

        unimplemented!("Interpreter is not implemented.");
    }

    /// Evaluates a library component
    /// # Arguments
    ///   * `c` - is the context for the compilation unit
    ///   * `st` - is the state of the component
    ///   * `id` - the id of component to run
    ///   * `inputs` - is a map of input port names to values for passing
    ///                inputs to the component during evaluation
    /// # Returns
    ///   Returns a map of output port names to values
    pub fn eval_lib(
        &self,
        st: State,
        id: &ast::Id,
        inputs: HashMap<ast::Id, Option<i64>>,
    ) -> Result<(HashMap<ast::Id, Option<i64>>, State), Error> {
        let comp = self.context.get_component(id)?;
        let params = comp.params;
        let mut outputs: HashMap<ast::Id, Option<i64>> = HashMap::new();

        if comp.name.to_string() == "std_const" {
            let valid = unwrap_input(&inputs, &comp.name, "valid")?;
            if valid == 1 {
                let p_width = params.get(0);
                let p_value = params.get(1);
                let mut out_value = None;
                if let (Some(value), Some(width)) = (p_value, p_width) {
                    let v = *value as i64;
                    let w = *width as u32;
                    let base: i64 = 2;

                    // Check that value is within bit width
                    if v > base.pow(w - 1) - 1 || v < -base.pow(w - 1) {
                        return Err(Error::InvalidConstant(
                            comp.name.to_string(),
                            v,
                            *width,
                        ));
                    }
                    out_value = Some(v)
                }

                outputs.insert(ast::Id::from("out"), out_value);
                outputs.insert(ast::Id::from("ready"), Some(1));
                return Ok((outputs, st.clone()));
            }

            // Valid not high- Default to outputting all None values
            outputs.insert(ast::Id::from("out"), None);
            outputs.insert(ast::Id::from("out_read_out"), None);
            outputs.insert(ast::Id::from("ready"), None);
            Ok((outputs, st.clone()))
        } else if comp.name.to_string() == "std_add" {
            let valid = unwrap_input(&inputs, &comp.name, "valid")?;
            let left = unwrap_input(&inputs, &comp.name, "left")?;
            let right = unwrap_input(&inputs, &comp.name, "left")?;
            if valid == 1 {
                let p_width = params.get(0);
                let mut out_value = None;
                if let Some(width) = p_width {
                    let v = left + right;
                    let w = *width as u32;
                    let base: i64 = 2;
                    // Check that value is within bit width
                    if v > base.pow(w - 1) - 1 || v < -base.pow(w - 1) {
                        return Err(Error::Overflow(comp.name.to_string()));
                    }
                    out_value = Some(v)
                }
                outputs.insert(ast::Id::from("out"), out_value);
                outputs.insert(ast::Id::from("ready"), Some(1));
                return Ok((outputs, st.clone()));
            }
            outputs.insert(ast::Id::from("out"), None);
            outputs.insert(ast::Id::from("out_read_out"), None);
            outputs.insert(ast::Id::from("ready"), None);
            Ok((outputs, st.clone()))
        } else if comp.name.to_string() == "std_reg" {
            let valid = unwrap_input(&inputs, &comp.name, "valid")?;
            let reg_in = unwrap_input(&inputs, &comp.name, "in")?;
            // Read output of register
            if valid == 1 {
                let p_width = params.get(0);
                let mut out_value = st.lookup_reg(id)?;
                if let Some(width) = p_width {
                    if let Some(v) = out_value {
                        let w = *width as u32;
                        let base: i64 = 2;
                        // Check that value is within bit width
                        if v > base.pow(w - 1) - 1 || v < -base.pow(w - 1) {
                            return Err(Error::Overflow(comp.name.to_string()));
                        }
                        out_value = Some(v)
                    }
                }
                println!("{:?}", out_value);
                outputs.insert(ast::Id::from("out"), out_value);
                outputs.insert(ast::Id::from("ready"), Some(1));
                // Return new state
                let st_1 = st.set_reg(id, Some(reg_in))?;
                return Ok((outputs, st_1));
            }
            outputs.insert(ast::Id::from("out"), None);
            outputs.insert(ast::Id::from("out_read_out"), None);
            outputs.insert(ast::Id::from("ready"), None);
            Ok((outputs, st))
        } else {
            Err(Error::UnimplementedPrimitive(comp.name))
        }
    }

    /// Simulates the control of a component
    /// # Arguments
    ///   * `c` - is the context for the compilation unit
    ///   * `inputs` - is a map of input port names to values for passing
    ///                inputs to the component during evaluation
    ///   * `control` - is the control statement to evaluate
    ///   * `structure` - is the graph of the structure
    ///   * `st` - is the state of the component
    ///   * `comp_name` - the name of the type of component to run
    /// # Returns
    ///   Returns the new component state
    pub fn eval_c(
        &self,
        inputs: &HashMap<ast::Id, Option<i64>>,
        control: &ast::Control,
        structure: &StructureGraph,
        st: &State,
    ) -> Result<State, Error> {
        use ast::Control;
        match control {
            Control::Seq { data } => {
                if data.stmts.is_empty() {
                    return Ok(st.clone());
                } else {
                    let (head, tail) = data.stmts.split_at(1);
                    let st_1 = self.eval_c(inputs, &head[0], structure, st)?;
                    let seq_1 = ast::Seq {
                        stmts: tail.to_vec(),
                    };
                    let control_1 = Control::Seq { data: seq_1 };
                    return self.eval_c(inputs, &control_1, structure, &st_1);
                }
            }
            Control::Par { data } => {
                unimplemented!("Parallel");
            }
            Control::If { data } => {
                unimplemented!("If");
            }
            Control::While { data } => {
                unimplemented!("While");
            }
            Control::Print { data } => {
                unimplemented!("Print");
            }
            Control::Enable { data } => {
                return self.eval_s(inputs, structure, st, data.comps.clone());
            }
            Control::Empty { data: _ } => return Ok(st.clone()),
        }
    }

    /// Simulates the structure of a component for `enable` statements
    /// # Arguments
    ///   * `c` - is the context for the compilation unit
    ///   * `inputs` - is a map of input port names to values for passing
    ///                inputs to the component during evaluation
    ///   * `structure` - is the graph of the structure
    ///   * `st` - is the context for the file
    ///   * `enabled` - is a list of enabled components to simulate
    /// # Returns
    ///   Returns the new component state
    pub fn eval_s(
        &self,
        inputs: &HashMap<ast::Id, Option<i64>>,
        structure: &StructureGraph,
        st: &State,
        enabled: Vec<ast::Id>,
    ) -> Result<State, Error> {
        // Create a fresh graph for evaluation so we don't impact the original structure
        // It should have no values on the wires
        let mut graph = structure.clone();
        graph.split_seq_prims(); // Split sequential primitives to remove valid cycles
        graph.drive_inputs(inputs); // Initialize with input values
        graph.drive_state(st); // Load values from State into graph
        let st_1 = graph.update(&self, st, enabled)?; // Simulate the hardware
        let output_map = graph.read_outputs();
        Ok(st_1)
    }
}

/// helper function to unwrap the hashmap result
fn unwrap_input(
    inputs: &HashMap<ast::Id, Option<i64>>,
    name: &ast::Id,
    port: &str,
) -> Result<i64, Error> {
    let input = inputs.get(&ast::Id::from(port));
    if let Some(Some(val)) = input {
        return Ok(*val);
    }
    Err(Error::MissingInput(name.to_string(), port.to_string()))
}
