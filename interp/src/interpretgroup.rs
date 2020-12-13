//! Used for the command line interface
//! Only interprets a given group in a given component

use super::interpreter;
use calyx::{
	errors::{Error, FutilResult},
	ir,
};

pub struct GroupInterpreter {
	// the name of the component
	pub component: String,
	// the name of the group to interpret
	pub group: String,
}

impl GroupInterpreter {
	// Returns the name of the interpreter
	pub fn name(self) -> String {
		"group interpreter".to_string()
	}

	// Interpret a group, given a context, component name, and group name
	pub fn interpret(self, ctx: &ir::Context) -> FutilResult<()> {
		// validation
		validate_names(ctx, &self.component, &self.group)?;

		// intialize environment
		let mut environment: interpreter::Environment = Default::default();
		let cells = get_cells(&ctx, &self.component);
		// interpret the group
		Ok(())
	}
}

// Ensures that the component and group names exist in the context
fn validate_names(
	ctx: &ir::Context,
	component: &String,
	group: &String,
) -> FutilResult<()> {
	let components = &ctx.clone().components;

	match components.into_iter().find(|&c| c.name.id == *component) {
		Some(comp) => {
			let groups = &comp.clone().groups;
			match groups.into_iter().find(|&g| g.borrow().name == *group) {
				Some(_) => Ok(()),
				None => Err(Error::UndefinedGroup(ir::Id::from(group.clone()))),
			}
		}
		None => Err(Error::UndefinedComponent(ir::Id::from(component.clone()))),
	}
}

// Find the component's cells in context; duplicated code?
fn get_cells(ctx: &ir::Context, component: &String) -> Vec<ir::RRC<ir::Cell>> {
	let components = &ctx.clone().components;
	match components.into_iter().find(|&c| c.name.id == *component) {
		Some(comp) => comp.cells.clone(),
		_ => panic!("This isn't supposed to happen?"),
	}
}

// Construct a map from id to cell
fn construct_cell_map() -> () {}
