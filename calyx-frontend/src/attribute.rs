use calyx_utils::{CalyxResult, Error, Id};

/// Attributes that have been deprecated.
pub const DEPRECATED_ATTRIBUTES: &[&str] = &[];

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
#[allow(non_camel_case_types)]
/// Defines the known attributes that can be attached to IR nodes.
pub enum Attribute {
    /// This is the top-level component
    TopLevel,
    /// Cell should be externalized
    External,
    /// The component doesn't have a standard interface
    NoInterface,
    // Interface attributes
    Go,
    Done,
    Reset,
    Clk,
    /// Latency information
    Static,
    /// Is the port connected to a state element
    Stable,
    /// The bound of a while loop
    Bound,
    // Interface properties
    ReadTogether,
    WriteTogether,
    /// Is this component shareable
    Share,
    /// Is the component state shareable
    StateShare,
    /// IR Node was generated by the compiler
    Generated,

    // Internal attributes. Not for public use and the frontend cannot parse them.
    NODE_ID,
    BEGIN_ID,
    END_ID,

    /// Unknown attribute. Should not appear in the Calyx codebase.
    /// Useful for other frontends using Calyx
    Unknown(Id),
}

impl From<Attribute> for Id {
    fn from(attr: Attribute) -> Id {
        match attr {
            Attribute::TopLevel => "toplevel".into(),
            Attribute::NoInterface => "no_interface".into(),
            Attribute::Go => "go".into(),
            Attribute::Done => "done".into(),
            Attribute::Reset => "reset".into(),
            Attribute::Clk => "clk".into(),
            Attribute::Static => "static".into(),
            Attribute::Stable => "stable".into(),
            Attribute::ReadTogether => "read_together".into(),
            Attribute::WriteTogether => "write_together".into(),
            Attribute::StateShare => "state_share".into(),
            Attribute::Share => "share".into(),
            Attribute::Generated => "generated".into(),
            Attribute::Bound => "bound".into(),
            Attribute::NODE_ID => "NODE_ID".into(),
            Attribute::BEGIN_ID => "BEGIN_ID".into(),
            Attribute::END_ID => "END_ID".into(),
            Attribute::External => "external".into(),
            Attribute::Unknown(s) => s,
        }
    }
}

impl ToString for Attribute {
    fn to_string(&self) -> String {
        Id::from(*self).to_string()
    }
}

impl TryFrom<String> for Attribute {
    type Error = Error;

    fn try_from(s: String) -> CalyxResult<Self> {
        if DEPRECATED_ATTRIBUTES.contains(&s.as_str()) {
            return Err(Error::malformed_structure(format!(
                "Attribute {s} is deprecated"
            )));
        }

        Ok(match s.as_str() {
            "toplevel" => Attribute::TopLevel,
            "external" => Attribute::External,
            "nointerface" => Attribute::NoInterface,
            "generated" => Attribute::Generated,
            "go" => Attribute::Go,
            "done" => Attribute::Done,
            "reset" => Attribute::Reset,
            "clk" => Attribute::Clk,
            "static" => Attribute::Static,
            "bound" => Attribute::Bound,
            "stable" => Attribute::Stable,
            "read_together" => Attribute::ReadTogether,
            "write_together" => Attribute::WriteTogether,
            "state_share" => Attribute::StateShare,
            "share" => Attribute::Share,
            _ => Attribute::Unknown(s.into()),
        })
    }
}
