use calyx_utils::{CalyxResult, Error, Id};
use std::str::FromStr;
use strum::EnumCount;
use strum_macros::{AsRefStr, EnumCount, EnumString, FromRepr};

/// Attributes that have been deprecated.
pub const DEPRECATED_ATTRIBUTES: &[&str] = &["static"];

#[derive(
    EnumCount,
    FromRepr,
    AsRefStr,
    EnumString,
    Clone,
    Copy,
    Hash,
    PartialEq,
    Eq,
    Debug,
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
#[repr(u8)]
/// Attributes that are only allowed to take boolean values.
pub enum BoolAttr {
    #[strum(serialize = "toplevel")]
    /// This is the top-level component
    TopLevel,
    #[strum(serialize = "external")]
    /// Cell should be externalized
    External,
    #[strum(serialize = "nointerface")]
    /// The component doesn't have a standard interface
    NoInterface,
    #[strum(serialize = "reset")]
    /// Reset signal for the component
    Reset,
    #[strum(serialize = "clk")]
    /// Clk for the signal
    Clk,
    #[strum(serialize = "stable")]
    /// Is the port connected to a state element
    Stable,
    #[strum(serialize = "data")]
    /// This is a data path instance
    Data,
    #[strum(serialize = "control")]
    /// This is a control path instance
    Control,
    #[strum(serialize = "share")]
    /// Is this component shareable
    Share,
    #[strum(serialize = "state_share")]
    /// Is the component state shareable
    StateShare,
    #[strum(serialize = "generated")]
    /// IR Node was generated by the compiler
    Generated,
    #[strum(serialize = "new_fsm")]
    /// Generate a new FSM for this control node
    NewFSM,
    #[strum(serialize = "one_hot")]
    /// Generate a one-hot FSM for this control node. (Not necesarily a
    /// guarantee: if the control node does not get its own FSM, then this attribute
    /// won't necesarily be honored.)
    OneHot,
    #[strum(serialize = "inline")]
    /// Inline this subcomponent
    Inline,
    #[strum(serialize = "promoted")]
    /// denotes a static component or control promoted from dynamic
    Promoted,
    #[strum(serialize = "par")]
    /// Denotes a group that was generated from a `staticpar` during static
    /// inlining.
    ParCtrl,
    #[strum(serialize = "fast")]
    /// https://github.com/calyxir/calyx/issues/1828
    Fast,
    #[strum(serialize = "protected")]
    /// Do I preserve this cell (and its uses) during optimization?
    Protected,
}

impl From<BoolAttr> for Attribute {
    fn from(attr: BoolAttr) -> Self {
        Attribute::Bool(attr)
    }
}
impl std::fmt::Display for BoolAttr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

#[derive(AsRefStr, EnumString, Clone, Copy, Hash, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
/// Attributes that can take numeric values
pub enum NumAttr {
    // ============ numeric attributes ============
    // Interface ports
    #[strum(serialize = "go")]
    Go,
    #[strum(serialize = "done")]
    Done,
    // Interface properties
    #[strum(serialize = "read_together")]
    ReadTogether,
    #[strum(serialize = "write_together")]
    WriteTogether,
    #[strum(serialize = "sync")]
    /// Synchronize this thread with others in the current par block
    Sync,
    #[strum(serialize = "bound")]
    /// The bound of a while loop
    Bound,
    #[strum(serialize = "pos")]
    /// Source location position for this node
    Pos,
    #[strum(serialize = "promotable")]
    /// Can promote the group, control, or @go port of the component to static
    /// with the annotated latency
    Promotable,
    #[strum(serialize = "compactable")]
    /// suggest that the current static seq block is compactable
    Compactable,
    #[strum(serialize = "interval")]
    /// Placed on @go ports of components to denote the II of a component, which
    /// is the same as the latency for non pipelined components.
    /// This indicates the component can serve ``double-duty'' as both static and
    /// dynamic.
    /// Therefore, we only place if we can *guarantee* the interval of the component.
    Interval,
}
impl From<NumAttr> for Attribute {
    fn from(attr: NumAttr) -> Self {
        Attribute::Num(attr)
    }
}
impl std::fmt::Display for NumAttr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

#[derive(AsRefStr, Clone, Copy, Hash, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
#[allow(non_camel_case_types)]
/// Internal attributes that cannot be parsed back from the IL.
pub enum InternalAttr {
    DEAD,
    NODE_ID,
    BEGIN_ID,
    END_ID,
    ST_ID,
    LOOP,
    START,
    END,
}
impl From<InternalAttr> for Attribute {
    fn from(attr: InternalAttr) -> Self {
        Attribute::Internal(attr)
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
/// Defines the known attributes that can be attached to IR nodes.
/// All caps names represent attributes that are internal to the compiler and
/// cannot be parsed back.
pub enum Attribute {
    Bool(BoolAttr),
    Num(NumAttr),
    Internal(InternalAttr),
    /// Unknown attribute. Should not appear in the Calyx codebase.
    /// Useful for other frontends using Calyx
    Unknown(Id),
}
impl std::fmt::Display for Attribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Attribute::Bool(b) => write!(f, "{}", b.as_ref()),
            Attribute::Num(n) => write!(f, "{}", n.as_ref()),
            Attribute::Internal(i) => write!(f, "{}", i.as_ref()),
            Attribute::Unknown(s) => write!(f, "{}", s),
        }
    }
}
impl FromStr for Attribute {
    type Err = Error;

    fn from_str(s: &str) -> CalyxResult<Self> {
        if let Ok(b) = BoolAttr::from_str(s) {
            Ok(Attribute::Bool(b))
        } else if let Ok(n) = NumAttr::from_str(s) {
            Ok(Attribute::Num(n))
        } else {
            if DEPRECATED_ATTRIBUTES.contains(&s) {
                log::warn!("The attribute @{s} is deprecated and will be ignored by the compiler.");
            }
            // Reject attributes that all caps since those are reserved for internal attributes
            if s.to_uppercase() == s {
                return Err(Error::misc(format!("Invalid attribute: {}. All caps attributes are reserved for internal use.", s)));
            }
            Ok(Attribute::Unknown(s.into()))
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
/// Inline storage for boolean attributes.
pub(super) struct InlineAttributes {
    /// Boolean attributes stored in a 32-bit number.
    attrs: u32,
}

impl InlineAttributes {
    /// Is the attribute set empty?
    pub const fn is_empty(&self) -> bool {
        self.attrs == 0
    }

    /// Adds an attribute to the set
    pub fn insert(&mut self, attr: BoolAttr) {
        self.attrs |= 1 << attr as u8;
    }

    /// Checks if the set contains an attribute
    pub fn has(&self, attr: BoolAttr) -> bool {
        self.attrs & (1 << (attr as u8)) != 0
    }

    /// Remove attribute from the set if present
    pub fn remove(&mut self, attr: BoolAttr) {
        self.attrs &= !(1 << attr as u8);
    }

    /// Returns an iterator over the attributes in the set
    pub(super) fn iter(&self) -> impl Iterator<Item = BoolAttr> + '_ {
        (0..(BoolAttr::COUNT as u8)).filter_map(|idx| {
            if self.attrs & (1 << idx) != 0 {
                Some(BoolAttr::from_repr(idx).unwrap())
            } else {
                None
            }
        })
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for InlineAttributes {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_owned().attrs.serialize(ser)
    }
}
