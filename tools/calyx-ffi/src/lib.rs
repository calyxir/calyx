use std::{any, cell::RefCell, collections::HashMap, rc::Rc};

pub mod backend;
pub mod prelude;

/// A non-combinational calyx component.
pub trait CalyxFFIComponent: any::Any {
    /// The in-source name of this component.
    fn name(&self) -> &'static str;

    /// Internal initialization routine. Do not call!
    fn init(&mut self);

    /// Internal deinitialization routine. Do not call!
    fn deinit(&mut self);

    // Resets this component.
    fn reset(&mut self);

    // Advances this component by one clock cycle. May not always be available.
    fn tick(&mut self);

    /// Calls this component.
    fn go(&mut self);
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
            let comp_ref = Rc::new(RefCell::new(T::default()));
            comp_ref.borrow_mut().init();
            self.comps.insert(name, comp_ref);
        }
        self.comps.get(name).unwrap().clone()
    }
}

impl Drop for CalyxFFI {
    fn drop(&mut self) {
        for (_, comp) in &self.comps {
            comp.borrow_mut().deinit();
        }
    }
}
