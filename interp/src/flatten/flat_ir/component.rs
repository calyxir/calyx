// #[derive(Debug)]
// pub struct ComponentDefinition {
//     /// Name of the component.
//     pub name: Id,
//     /// The input/output signature of this component.
//     pub signature: RRC<Cell>,
//     /// The cells instantiated for this component.
//     pub cells: IdList<Cell>,
//     /// Groups of assignment wires.
//     pub groups: IdList<Group>,
//     /// Groups of assignment wires.
//     pub comb_groups: IdList<CombGroup>,
//     /// The set of "continuous assignments", i.e., assignments that are always
//     /// active.
//     pub continuous_assignments: Vec<Assignment>,
//     /// The control program for this component.
//     pub control: RRC<Control>,
//     /// Attributes for this component
//     pub attributes: Attributes,
//     /// True iff component is combinational
//     pub is_comb: bool,
// }
