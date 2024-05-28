use std::{cell::RefCell, path::PathBuf, sync::RwLock};

use allocative::Allocative;
use anyhow::anyhow;
use fud_core::{
    exec::{OpRef, StateRef},
    run::{EmitResult, EmitSetup, Emitter},
    DriverBuilder,
};
use starlark::{
    environment::{
        GlobalsBuilder, LibraryExtension, Methods, MethodsBuilder,
        MethodsStatic, Module,
    },
    eval::Evaluator,
    starlark_module,
    syntax::{AstModule, Dialect},
    values::{
        list::UnpackList, none::NoneType, starlark_value, AllocValue,
        NoSerialize, ProvidesStaticType, StarlarkValue, UnpackValue, Value,
    },
};

#[derive(ProvidesStaticType)]
struct DbWrap(RefCell<DriverBuilder>);

impl DbWrap {
    fn new(driver_builder: DriverBuilder) -> Self {
        Self(RefCell::new(driver_builder))
    }
}

#[derive(Debug, ProvidesStaticType, Allocative, Clone)]
enum Stmt {
    ConfigVar(String, String),
    Rule(String, String),
}

impl Stmt {
    fn exec(&self, e: &mut Emitter) -> EmitResult {
        match self {
            Stmt::ConfigVar(name, key) => Ok(e.config_var(name, key)?),
            Stmt::Rule(name, command) => Ok(e.rule(name, command)?),
        }
    }
}

#[derive(Debug, ProvidesStaticType, Default, Allocative, Clone)]
struct EmitterStmts(Vec<Stmt>);

impl EmitterStmts {
    fn push(&mut self, item: Stmt) {
        self.0.push(item);
    }
}

impl EmitSetup for EmitterStmts {
    fn setup(&self, e: &mut Emitter) -> EmitResult {
        self.0.iter().try_fold((), |_, it| it.exec(e))
    }
}

#[derive(
    Debug, derive_more::Display, Allocative, NoSerialize, ProvidesStaticType,
)]
struct StarlarkStateRef(#[allocative(skip)] StateRef);
#[starlark_value(type = "StateRef", UnpackValue, StarlarkTypeRepr)]
impl<'v> StarlarkValue<'v> for StarlarkStateRef {}
impl<'v> AllocValue<'v> for StarlarkStateRef {
    fn alloc_value(self, heap: &'v starlark::values::Heap) -> Value<'v> {
        heap.alloc_simple(self)
    }
}

#[derive(
    Debug, derive_more::Display, Allocative, NoSerialize, ProvidesStaticType,
)]
struct StarlarkOpRef(#[allocative(skip)] OpRef);
#[starlark_value(type = "OpRef", UnpackValue, StarlarkTypeRepr)]
impl<'v> StarlarkValue<'v> for StarlarkOpRef {}
impl<'v> AllocValue<'v> for StarlarkOpRef {
    fn alloc_value(self, heap: &'v starlark::values::Heap) -> Value<'v> {
        heap.alloc_simple(self)
    }
}

#[derive(
    Debug,
    ProvidesStaticType,
    Default,
    Allocative,
    NoSerialize,
    derive_more::Display,
)]
#[display(fmt = "EmitedStore({:?})", _0)]
struct EmitterStore(RwLock<EmitterStmts>);

#[starlark_module]
fn emit_methods(builder: &mut MethodsBuilder) {
    fn config_var(
        this: &EmitterStore,
        name: &str,
        key: &str,
    ) -> anyhow::Result<NoneType> {
        this.0
            .write()
            .unwrap()
            .push(Stmt::ConfigVar(name.to_string(), key.to_string()));
        Ok(NoneType)
    }

    fn rule(
        this: &EmitterStore,
        name: &str,
        command: &str,
    ) -> anyhow::Result<NoneType> {
        this.0
            .write()
            .unwrap()
            .push(Stmt::Rule(name.to_string(), command.to_string()));
        Ok(NoneType)
    }
}

/// This is how we inject methods onto the `EmitterStore` struct
#[starlark_value(type = "EmitterStore", UnpackValue, StarlarkTypeRepr)]
impl<'v> StarlarkValue<'v> for EmitterStore {
    fn get_methods() -> Option<&'static Methods> {
        static RES: MethodsStatic = MethodsStatic::new();
        RES.methods(emit_methods)
    }
}

#[starlark_module]
fn global_dfns(builder: &mut GlobalsBuilder) {
    fn state(
        name: &str,
        extensions: UnpackList<&str>,
        eval: &mut Evaluator,
    ) -> anyhow::Result<StarlarkStateRef> {
        let mut db_ref = eval
            .extra
            .unwrap()
            .downcast_ref::<DbWrap>()
            .unwrap()
            .0
            .borrow_mut();
        Ok(StarlarkStateRef(db_ref.state(name, &extensions.items)))
    }

    fn get_state(
        name: &str,
        eval: &mut Evaluator,
    ) -> anyhow::Result<StarlarkStateRef> {
        let db_ref = eval
            .extra
            .unwrap()
            .downcast_ref::<DbWrap>()
            .unwrap()
            .0
            .borrow();

        db_ref
            .find_state(name)
            .map(StarlarkStateRef)
            .ok_or(anyhow!("Unknown state: {name}"))
    }

    fn rule<'v>(
        setups: UnpackList<Value<'v>>,
        input: &StarlarkStateRef,
        output: &StarlarkStateRef,
        rule_name: &str,
        eval: &mut Evaluator<'v, '_>,
    ) -> anyhow::Result<StarlarkOpRef> {
        let mut db_ref = eval
            .extra
            .unwrap()
            .downcast_ref::<DbWrap>()
            .unwrap()
            .0
            .borrow_mut();

        // get setuprefs for starlark functions
        let heap = eval.heap();
        let mut stores = vec![];
        for setup in setups {
            let store: &EmitterStore =
                <&EmitterStore as UnpackValue>::unpack_value(
                    eval.eval_function(
                        setup,
                        &[heap.alloc_simple(EmitterStore::default())],
                        &[],
                    )
                    .unwrap(),
                )
                .unwrap();
            let store_ref = store.0.read().unwrap();
            stores.push(store_ref.clone());
        }

        let setup_refs = stores
            .into_iter()
            .map(|s| db_ref.add_setup("test", s))
            .collect::<Vec<_>>();

        Ok(StarlarkOpRef(db_ref.rule(
            &setup_refs,
            input.0,
            output.0,
            rule_name,
        )))
    }
}

// todo: would like to change this to just take a &mut DriverBuilder. I'll play around with that later
pub fn build_plugins(bld: DriverBuilder, paths: &[PathBuf]) -> DriverBuilder {
    let dbwrap = DbWrap::new(bld);

    let globals = GlobalsBuilder::extended_by(&[LibraryExtension::Print])
        .with(global_dfns)
        .build();
    let module = Module::new();
    let mut eval = Evaluator::new(&module);

    // we have to use unsafe here to construct a reference to `bld`
    // this is because `Evaluator` forces the lifetime of the reference
    // passed in for `eval.extra` to live as long as `bld`. This makes
    // it impossible to use `bld` again, because the reference might
    // still be alive. There might be another way around this, but this
    // is what I could find
    let bld_ref: *const DbWrap = &dbwrap as *const _;
    eval.extra = Some(unsafe { bld_ref.as_ref().unwrap() });

    for p in paths {
        let ast = AstModule::parse_file(&p, &Dialect::Standard).unwrap();
        eval.eval_module(ast, &globals).unwrap();
    }

    // extract the driver builder out of the refcell
    dbwrap.0.into_inner()
}
