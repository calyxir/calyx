use super::Visitor;
use calyx_ir as ir;
use calyx_utils::{CalyxResult, OutputFile};
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;
use std::iter;

#[derive(Clone)]
/// The value returned from parsing an option.
pub enum ParseVal {
    /// A boolean option.
    Bool(bool),
    /// A number option.
    Num(i64),
    /// A list of values.
    List(Vec<ParseVal>),
    /// An output stream (stdout, stderr, file name)
    OutStream(OutputFile),
}

impl ParseVal {
    pub fn bool(&self) -> bool {
        let ParseVal::Bool(b) = self else {
            panic!("Expected bool, got {self}");
        };
        *b
    }

    pub fn num(&self) -> i64 {
        let ParseVal::Num(n) = self else {
            panic!("Expected number, got {self}");
        };
        *n
    }

    pub fn pos_num(&self) -> Option<u64> {
        let n = self.num();
        if n < 0 {
            None
        } else {
            Some(n as u64)
        }
    }

    pub fn num_list(&self) -> Vec<i64> {
        match self {
            ParseVal::List(l) => {
                l.iter().map(ParseVal::num).collect::<Vec<_>>()
            }
            _ => panic!("Expected list of numbers, got {self}"),
        }
    }

    /// Parse a list that should have exactly N elements. If elements are missing, then add None
    /// to the end of the list.
    pub fn num_list_exact<const N: usize>(&self) -> [Option<i64>; N] {
        let list = self.num_list();
        let len = list.len();
        if len > N {
            panic!("Expected list of {N} numbers, got {len}");
        }
        list.into_iter()
            .map(Some)
            .chain(iter::repeat(None).take(N - len))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }

    /// Returns an output stream if it is not the null stream
    pub fn not_null_outstream(&self) -> Option<OutputFile> {
        match self {
            ParseVal::OutStream(o) => {
                if matches!(o, OutputFile::Null) {
                    None
                } else {
                    Some(o.clone())
                }
            }
            _ => panic!("Expected output stream, got {self}"),
        }
    }
}
impl std::fmt::Display for ParseVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseVal::Bool(b) => write!(f, "{b}"),
            ParseVal::Num(n) => write!(f, "{n}"),
            ParseVal::List(l) => {
                write!(f, "[")?;
                for (i, e) in l.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{e}")?;
                }
                write!(f, "]")
            }
            ParseVal::OutStream(o) => write!(f, "{}", o.to_string()),
        }
    }
}

/// Option that can be passed to a pass.
pub struct PassOpt {
    name: &'static str,
    description: &'static str,
    default: ParseVal,
    parse: fn(&str) -> Option<ParseVal>,
}

impl PassOpt {
    pub const fn new(
        name: &'static str,
        description: &'static str,
        default: ParseVal,
        parse: fn(&str) -> Option<ParseVal>,
    ) -> Self {
        Self {
            name,
            description,
            default,
            parse,
        }
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub const fn description(&self) -> &'static str {
        self.description
    }

    pub const fn default(&self) -> &ParseVal {
        &self.default
    }

    fn parse(&self, s: &str) -> Option<ParseVal> {
        (self.parse)(s)
    }

    /// Parse of list using parser for the elements.
    /// Returns `None` if any of the elements fail to parse.
    fn parse_list(
        s: &str,
        parse: fn(&str) -> Option<ParseVal>,
    ) -> Option<ParseVal> {
        let mut res = Vec::new();
        for e in s.split(',') {
            res.push(parse(e)?);
        }
        Some(ParseVal::List(res))
    }

    pub fn parse_bool(s: &str) -> Option<ParseVal> {
        match s {
            "true" => Some(ParseVal::Bool(true)),
            "false" => Some(ParseVal::Bool(false)),
            _ => None,
        }
    }

    /// Parse a number from a string.
    pub fn parse_num(s: &str) -> Option<ParseVal> {
        s.parse::<i64>().ok().map(ParseVal::Num)
    }

    /// Parse a list of numbers from a string.
    pub fn parse_num_list(s: &str) -> Option<ParseVal> {
        Self::parse_list(s, Self::parse_num)
    }

    pub fn parse_outstream(s: &str) -> Option<ParseVal> {
        s.parse::<OutputFile>().ok().map(ParseVal::OutStream)
    }
}

/// Trait that describes named things. Calling [`do_pass`](Visitor::do_pass) and [`do_pass_default`](Visitor::do_pass_default).
/// require this to be implemented.
///
/// This has to be a separate trait from [`Visitor`] because these methods don't recieve `self` which
/// means that it is impossible to create dynamic trait objects.
pub trait Named {
    /// The name of a pass. Is used for identifying passes.
    fn name() -> &'static str;
    /// A short description of the pass.
    fn description() -> &'static str;
    /// Set of options that can be passed to the pass.
    /// The options contains a tuple of the option name and a description.
    fn opts() -> Vec<PassOpt> {
        vec![]
    }
}

/// Trait defining method that can be used to construct a Visitor from an
/// [ir::Context].
/// This is useful when a pass needs to construct information using the context
/// *before* visiting the components.
///
/// For passes that don't need to use the context, this trait can be automatically
/// be derived from [Default].
pub trait ConstructVisitor {
    fn get_opts(ctx: &ir::Context) -> LinkedHashMap<&'static str, ParseVal>
    where
        Self: Named,
    {
        let opts = Self::opts();
        let n = Self::name();
        let mut values: LinkedHashMap<&'static str, ParseVal> = ctx
            .extra_opts
            .iter()
            .filter_map(|opt| {
                // The format is either -x pass:opt or -x pass:opt=val
                let mut splits = opt.split(':');
                if let Some(pass) = splits.next() {
                    if pass == n {
                        let mut splits = splits.next()?.split('=');
                        let opt = splits.next()?.to_string();
                        let Some(opt) = opts.iter().find(|o| o.name == opt) else {
                            log::warn!("Ignoring unknown option for pass `{n}`: {opt}");
                                return None;
                        };
                        let val = if let Some(v) = splits.next() {
                            let Some(v) = opt.parse(v) else {
                                log::warn!(
                                    "Ignoring invalid value for option `{n}:{}`: {v}",
                                    opt.name(),
                                );
                                return None;
                            };
                            v
                        } else {
                            ParseVal::Bool(true)
                        };
                        return Some((opt.name(), val));
                    }
                }
                None
            })
            .collect();

        if log::log_enabled!(log::Level::Debug) {
            log::debug!(
                "Extra options for {}: {}",
                Self::name(),
                values.iter().map(|(o, v)| format!("{o}->{v}")).join(", ")
            );
        }

        // For all options that were not provided with values, fill in the defaults.
        for opt in opts {
            if !values.contains_key(opt.name()) {
                values.insert(opt.name(), opt.default.clone());
            }
        }

        values
    }

    /// Construct the visitor using information from the Context
    fn from(_ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized;

    /// Clear the data stored in the visitor. Called before traversing the
    /// next component by [ir::traversal::Visitor].
    fn clear_data(&mut self);
}

/// Derive ConstructVisitor when [Default] is provided for a visitor.
impl<T: Default + Sized + Visitor> ConstructVisitor for T {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self> {
        Ok(T::default())
    }

    fn clear_data(&mut self) {
        *self = T::default();
    }
}
