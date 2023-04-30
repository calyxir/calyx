use calyx_utils::{CalyxResult, Error};

/// Attributes that have been deprecated.
pub const DEPRECATED: &[&str] = &[];

#[derive(Clone, Hash, PartialEq, Eq)]
/// Defines the known attributes that can be attached to IR nodes.
pub enum Attribute {
    // Interface attributes
    Go,
    Done,
    Reset,
    Clk,
    /// Latency information
    Static,
    /// Is the port connected to a state element
    Stable,
    // Interface properties
    ReadTogether,
    WriteTogether,
    /// Is the component state shareable
    StateShare,
    /// Unknown attribute. Should not appear in the Calyx codebase.
    /// Useful for other frontends using Calyx
    Unknown(Box<String>),
}

impl TryFrom<String> for Attribute {
    type Error = Error;

    fn try_from(s: String) -> CalyxResult<Self> {
        if DEPRECATED.contains(&s.as_str()) {
            return Err(Error::malformed_structure(format!(
                "Attribute {s} is deprecated"
            )));
        }

        Ok(match s.as_str() {
            "go" => Attribute::Go,
            "done" => Attribute::Done,
            "reset" => Attribute::Reset,
            "clk" => Attribute::Clk,
            "static" => Attribute::Static,
            "stable" => Attribute::Stable,
            "read_together" => Attribute::ReadTogether,
            "write_together" => Attribute::WriteTogether,
            "state_share" => Attribute::StateShare,
            _ => Attribute::Unknown(Box::new(s)),
        })
    }
}
