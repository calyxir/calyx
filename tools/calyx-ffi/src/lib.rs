use calyx_ir::Context;
use std::{
    any, cell::RefCell, collections::HashMap, env, path::PathBuf, rc::Rc,
};

pub mod backend;
pub mod prelude;

/// A non-combinational calyx component.
pub trait CalyxFFIComponent: any::Any {
    /// The path to the component source file. Must be a constant expression.
    fn path(&self) -> &'static str;

    /// The in-source name of this component. Must be a constant expression.
    fn name(&self) -> &'static str;

    /// Internal initialization routine. Do not call!
    fn init(&mut self, context: &Context);

    /// Resets this component.
    fn reset(&mut self);

    /// Whether this component's backend supports ticking.
    fn can_tick(&self) -> bool;

    /// Advances this component by one clock cycle. May not always be available, so check [`has_tick`]([CalyxFFIComponent::has_tick]).
    fn tick(&mut self);

    /// Calls this component, blocking until it is done executing.
    fn go(&mut self);
}

pub type CalyxFFIComponentRef = Rc<RefCell<dyn CalyxFFIComponent>>;

fn box_calyx_ffi_component<T: CalyxFFIComponent>(
    comp: T,
) -> CalyxFFIComponentRef {
    Rc::new(RefCell::new(comp))
}

#[derive(Default)]
pub struct CalyxFFI {
    contexts: HashMap<&'static str, Context>,
}

impl CalyxFFI {
    pub fn new() -> Self {
        Self::default()
    }

    /// Constructs a new calyx component of the given type.
    ///
    /// The `path` implementation for `CalyxFFIComponent` must be a constant
    /// expression and should derived via the `calyx_ffi` procedural macro.
    pub fn new_comp<T: CalyxFFIComponent + Default>(
        &mut self,
    ) -> CalyxFFIComponentRef {
        let mut comp = T::default();
        let path = comp.path();
        let context = self.contexts.entry(path).or_insert_with_key(|path| {
            // there has to be a better way to find lib
            let home_dir = env::var("HOME").expect("user home not set");
            let mut lib_path = PathBuf::from(home_dir);
            lib_path.push(".calyx");
            let ws = calyx_frontend::Workspace::construct(
                &Some(path.into()),
                &lib_path,
            )
            .expect("couldn't parse calyx");
            calyx_ir::from_ast::ast_to_ir(ws)
                .expect("couldn't construct calyx ir")
        });
        comp.init(context);
        box_calyx_ffi_component(comp)
    }
}

pub type Value<const N: u64> = interp::BitVecValue;

pub fn value_from_u64<const N: u64>(value: u64) -> Value<N> {
    Value::from_u64(value, N as u32)
}
