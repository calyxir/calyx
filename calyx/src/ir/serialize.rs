
use crate::ir::context;
use serde::{ser::SerializeStruct, Serialize, Serializer};

impl Serialize for Context {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ctx = ser.serialize_struct("Context", 1)?;
        ctx.serialize_field("components", &self.components)?;
        /*
        ctx.serialize_field("components", &self.components)?;
        ctx.serialize_field("lib", &self.lib)?;
        ctx.serialize_field("entrypoint", &self.entrypoint)?;
        ctx.serialize_field("bc", &self.bc)?;
        ctx.serialize_field("extra_opts", &self.extra_opts)?;
        ctx.serialize_field("metadata", &self.metadata)?;
        */
        ctx.end()
    }
}
