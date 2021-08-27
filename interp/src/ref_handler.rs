use crate::utils::AsRaw;
use calyx::ir::{self, Component, Context, Control, RRC};
use std::collections::HashMap;
pub struct RefHandler<'a> {
    map: HashMap<*const Component, (&'a Component, &'a Control)>,
}

impl<'a> RefHandler<'a> {
    pub fn construct<
        I1: Iterator<Item = &'a Component>,
        I2: Iterator<Item = &'a Control>,
    >(
        comps: I1,
        controls: I2,
    ) -> Self {
        let map = comps
            .zip(controls)
            .map(|(comp, control)| (comp as *const Component, (comp, control)))
            .collect();
        Self { map }
    }

    pub fn get<C: AsRaw<Component>>(&self, comp: C) -> (&Component, &Control) {
        self.map[&comp.as_raw()]
    }

    pub fn get_by_name<S: AsRef<str>>(
        &self,
        name: S,
    ) -> (&Component, &Control) {
        *(self
            .map
            .values()
            .find(|(comp, _)| comp.name == name)
            .unwrap())
    }
}
