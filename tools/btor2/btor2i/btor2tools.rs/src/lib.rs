//! The Btor2Tools package provides a generic parser and tools for the BTOR2 format.
//!
//! This crate provides high-level FFI bindings for the [C Btor2Tools package](https://github.com/Boolector/btor2tools).
//! For a more detailed description of the BTOR2 format, refer to BTOR2, BtorMC and Boolector 3.0. Aina Niemetz, Mathias Preiner, Clifford Wolf, and Armin Biere. CAV 2018.
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//! use btor2tools::Btor2Parser;
//!
//! let btor2_file = Path::new("example.btor2");
//!
//! Btor2Parser::new()
//!     .read_lines(&btor2_file)
//!     .unwrap() // ignore parser error
//!     .for_each(|line| {
//!         // print every parsed line
//!         println!("{:?}", line);
//!     });
//! ```

use btor2tools_sys::{
    btor2parser_delete, btor2parser_error, btor2parser_iter_init, btor2parser_iter_next,
    btor2parser_new, btor2parser_read_lines, fclose, fopen, Btor2Line as CBtor2Line,
    Btor2LineIterator as CBtor2LineIterator, Btor2Parser as CBtor2Parser,
    Btor2SortTag as CBtor2SortTag, Btor2Tag as CBtor2Tag,
};
use std::{
    convert::From,
    ffi::{CStr, CString},
    fmt,
    marker::PhantomData,
    os::raw::c_char,
    path::Path,
    slice,
};
use thiserror::Error;

pub struct Btor2Parser {
    internal: *mut CBtor2Parser,
}

impl Btor2Parser {
    pub fn new() -> Self {
        Self {
            internal: unsafe { btor2parser_new() },
        }
    }

    /// Parses a Btor2 file and returns an iterator to all every formatted line on success.
    /// On failure, the error includes the line number, where the error occured.
    pub fn read_lines<P>(&mut self, file: P) -> Result<Btor2LineIterator, Btor2ParserError>
    where
        P: AsRef<Path>,
    {
        unsafe {
            let file_path = if let Some(p) = file.as_ref().to_str() {
                p
            } else {
                return Err(Btor2ParserError::InvalidPathEncoding(String::from(
                    "Path is not UTF-8 encoded",
                )));
            };

            let c_file_path = CString::new(file_path).map_err(|_| {
                Btor2ParserError::InvalidPathEncoding(String::from(
                    "Path contains a illegal 0 byte",
                ))
            })?;

            let c_file_mode = CString::new("r").unwrap();

            let file = fopen(c_file_path.as_ptr(), c_file_mode.as_ptr());

            if file.is_null() {
                Err(Btor2ParserError::CouldNotOpenFile(file_path.to_owned()))
            } else {
                let result = btor2parser_read_lines(self.internal, file);

                fclose(file);

                if result == 0 {
                    let c_msg = CStr::from_ptr(btor2parser_error(self.internal));

                    Err(Btor2ParserError::SyntaxError(
                        c_msg
                            .to_str()
                            .expect("Btor2tools do not use valid UTF-8 strings")
                            .to_owned(),
                    ))
                } else {
                    Ok(Btor2LineIterator::new(self))
                }
            }
        }
    }
}

impl Default for Btor2Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Btor2Parser {
    fn drop(&mut self) {
        unsafe { btor2parser_delete(self.internal) }
    }
}

#[derive(Error, Debug)]
pub enum Btor2ParserError {
    #[error("Could not open file: {0}")]
    CouldNotOpenFile(String),

    #[error("BTOR2 syntax error in {0}")]
    SyntaxError(String),

    #[error("File path violates encoding rules: {0}")]
    InvalidPathEncoding(String),
}

#[derive(Copy, Clone)]
pub struct Btor2LineIterator<'parser> {
    parser: PhantomData<&'parser Btor2Parser>,
    internal: CBtor2LineIterator,
}

impl<'parser> Btor2LineIterator<'parser> {
    fn new(parser: &'parser Btor2Parser) -> Self {
        Self {
            parser: PhantomData,
            internal: unsafe { btor2parser_iter_init(parser.internal) },
        }
    }
}

impl<'parser> Iterator for Btor2LineIterator<'parser> {
    type Item = Btor2Line<'parser>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let c_line = btor2parser_iter_next(&mut self.internal);

            if c_line.is_null() {
                None
            } else {
                Some(Btor2Line::new(c_line))
            }
        }
    }
}

#[derive(Clone)]
pub struct Btor2Line<'parser> {
    parser: PhantomData<&'parser Btor2Parser>,
    internal: *const CBtor2Line,
}

impl<'parser> Btor2Line<'parser> {
    fn new(internal: *mut CBtor2Line) -> Self {
        Self {
            parser: PhantomData,
            internal,
        }
    }

    /// positive id (non zero)
    pub fn id(&self) -> i64 {
        unsafe { (*self.internal).id }
    }

    /// line number in original file
    pub fn lineno(&self) -> i64 {
        unsafe { (*self.internal).lineno }
    }

    /// name in ASCII: "and", "add",...
    pub fn name(&self) -> &CStr {
        unsafe { CStr::from_ptr((*self.internal).name) }
    }

    /// same as name but encoded as enum
    pub fn tag(&self) -> Btor2Tag {
        unsafe { Btor2Tag::from((*self.internal).tag) }
    }

    pub fn sort(&self) -> Btor2Sort {
        Btor2Sort {
            line: PhantomData,
            internal: self.internal,
        }
    }

    /// non zero if initialized or has next
    pub fn init(&self) -> i64 {
        unsafe { (*self.internal).init }
    }

    /// non zero if initialized or has next
    pub fn next(&self) -> i64 {
        unsafe { (*self.internal).next }
    }

    /// non zero for const, constd, consth
    pub fn constant(&self) -> Option<&CStr> {
        wrap_nullable_c_string(unsafe { (*self.internal).constant })
    }

    /// optional for: var array state input
    pub fn symbol(&self) -> Option<&CStr> {
        wrap_nullable_c_string(unsafe { (*self.internal).symbol })
    }

    // non zero ids
    pub fn args(&self) -> &[i64] {
        unsafe { slice::from_raw_parts((*self.internal).args, (*self.internal).margs as usize) }
    }
}

impl<'parser> fmt::Debug for Btor2Line<'parser> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Btor2Line")
            .field("id", &self.id())
            .field("lineno", &self.lineno())
            .field("name", &self.name())
            .field("tag", &self.tag())
            .field("sort", &self.sort())
            .field("init", &self.init())
            .field("next", &self.next())
            .field("constant", &self.constant())
            .field("symbol", &self.symbol())
            .field("args", &self.args())
            .finish()
    }
}

#[derive(Copy, Clone)]
pub struct Btor2Sort<'line, 'parser> {
    line: PhantomData<&'line Btor2Line<'parser>>,
    internal: *const CBtor2Line,
}

impl<'line, 'parser> Btor2Sort<'line, 'parser> {
    pub fn id(&self) -> i64 {
        unsafe { (*self.internal).sort.id }
    }

    pub fn tag(&self) -> Btor2SortTag {
        unsafe { Btor2SortTag::from((*self.internal).sort.tag) }
    }

    pub fn name(&self) -> Option<&CStr> {
        wrap_nullable_c_string(unsafe { (*self.internal).sort.name })
    }

    pub fn content(&self) -> Btor2SortContent {
        unsafe {
            match self.tag() {
                Btor2SortTag::Array => Btor2SortContent::Array {
                    index: (*self.internal).sort.__bindgen_anon_1.array.index,
                    element: (*self.internal).sort.__bindgen_anon_1.array.element,
                },
                Btor2SortTag::Bitvec => Btor2SortContent::Bitvec {
                    width: (*self.internal).sort.__bindgen_anon_1.bitvec.width,
                },
            }
        }
    }
}

impl<'line, 'parser> fmt::Debug for Btor2Sort<'line, 'parser> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Btor2Sort")
            .field("id", &self.id())
            .field("tag", &self.tag())
            .field("name", &self.name())
            .field("content", &self.content())
            .finish()
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Btor2SortContent {
    Array { index: i64, element: i64 },
    Bitvec { width: u32 },
}

/// BTOR2 tags can be used for fast(er) traversal and operations on BTOR2
/// format lines, e.g., in a switch statement in client code.
/// Alternatively, client code can use the name of the BTOR2 tag, which is a C
/// string (redundantly) contained in the format line. Note that this requires
/// string comparisons and is therefore slower even if client code uses an
/// additional hash table.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub enum Btor2Tag {
    Add,
    And,
    Bad,
    Concat,
    Const,
    Constraint,
    Constd,
    Consth,
    Dec,
    Eq,
    Fair,
    Iff,
    Implies,
    Inc,
    Init,
    Input,
    Ite,
    Justice,
    Mul,
    Nand,
    Neq,
    Neg,
    Next,
    Nor,
    Not,
    One,
    Ones,
    Or,
    Output,
    Read,
    Redand,
    Redor,
    Redxor,
    Rol,
    Ror,
    Saddo,
    Sdiv,
    Sdivo,
    Sext,
    Sgt,
    Sgte,
    Slice,
    Sll,
    Slt,
    Slte,
    Sort,
    Smod,
    Smulo,
    Sra,
    Srem,
    Srl,
    Ssubo,
    State,
    Sub,
    Uaddo,
    Udiv,
    Uext,
    Ugt,
    Ugte,
    Ult,
    Ulte,
    Umulo,
    Urem,
    Usubo,
    Write,
    Xnor,
    Xor,
    Zero,
}

impl From<CBtor2Tag> for Btor2Tag {
    fn from(raw: CBtor2Tag) -> Btor2Tag {
        unsafe { core::mem::transmute(raw) }
    }
}

impl Into<CBtor2Tag> for Btor2Tag {
    fn into(self) -> CBtor2Tag {
        unsafe { core::mem::transmute(self) }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub enum Btor2SortTag {
    Array,
    Bitvec,
}

impl From<CBtor2SortTag> for Btor2SortTag {
    fn from(raw: CBtor2SortTag) -> Btor2SortTag {
        unsafe { std::mem::transmute(raw) }
    }
}

impl Into<CBtor2SortTag> for Btor2SortTag {
    fn into(self) -> CBtor2SortTag {
        unsafe { std::mem::transmute(self) }
    }
}

fn wrap_nullable_c_string<'a>(str: *const c_char) -> Option<&'a CStr> {
    unsafe {
        if str.is_null() {
            None
        } else {
            Some(CStr::from_ptr(str))
        }
    }
}
