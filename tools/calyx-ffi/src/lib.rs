use std::{any, cell::RefCell, collections::HashMap, rc::Rc};

pub mod prelude;

/// A non-combinational calyx component.
pub trait CalyxFFIComponent: any::Any {
    /// The in-source name of this component.
    fn name(&self) -> &'static str;

    // Resets this component.
    fn reset(&mut self);

    // Advances this component by one clock cycle.
    fn tick(&mut self);
}

pub type CalyxFFIComponentRef = Rc<RefCell<dyn CalyxFFIComponent>>;

#[derive(Default)]
pub struct CalyxFFI {
    comps: HashMap<&'static str, CalyxFFIComponentRef>,
}

impl CalyxFFI {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn comp<T: CalyxFFIComponent + Default>(
        &mut self,
    ) -> CalyxFFIComponentRef {
        let name = T::default().name();
        if !self.comps.contains_key(name) {
            self.comps.insert(name, Rc::new(RefCell::new(T::default())));
        }
        self.comps.get(name).unwrap().clone()
    }
}
