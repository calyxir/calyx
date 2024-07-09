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
    reuse: HashMap<&'static str, usize>,
    comps: Vec<CalyxFFIComponentRef>,
}

impl CalyxFFI {
    pub fn new() -> Self {
        Self::default()
    }

    /// Any component `T`.
    pub fn comp<T: CalyxFFIComponent + Default>(
        &mut self,
    ) -> CalyxFFIComponentRef {
        let name = T::default().name();
        if let Some(index) = self.reuse.get(name) {
            self.comps[*index].clone()
        } else {
            self.new_comp::<T>()
        }
    }

    /// A new component `T`.
    pub fn new_comp<T: CalyxFFIComponent + Default>(
        &mut self,
    ) -> CalyxFFIComponentRef {
        let comp = Rc::new(RefCell::new(T::default()));
        comp.borrow_mut().init();
        self.comps.push(comp.clone());
        self.reuse
            .insert(comp.borrow().name(), self.comps.len() - 1);
        comp
    }
}

impl Drop for CalyxFFI {
    fn drop(&mut self) {
        for comp in &self.comps {
            comp.borrow_mut().deinit();
        }
    }
}
