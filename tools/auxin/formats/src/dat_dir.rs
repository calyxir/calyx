use crate::filerep::*;
use baa::BitVecOps;
use num_ir::memrep::SingleMem;
use num_ir::typing::Endian;
use std::io::{BufRead, Write};

use smallvec::smallvec;

// some use of https://nnethercote.github.io/perf-book/heap-allocations.html#reading-lines-from-a-file throughout. it does actually have a measurable impact.

use std::collections::HashMap;

use num_ir::{short::TryFromShort, typing::TypeSpec};

pub struct HeadEntry {
    pub ty: TypeSpec,
    pub len: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum DirError {
    #[error("can't read {0}")]
    Header(String),
    #[error("header int parsing: {0}")]
    IntParse(#[from] std::num::ParseIntError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("short type: {0}")]
    ShortT(#[from] num_ir::short::ShortTypeErr),
    #[error("num_ir parse: {0}")]
    NumIr(#[from] num_ir::typing::NumParseErr),
}

/// read functionality for a plain text directory header format.
///
/// format: ``name,short_t,len\n``
///
/// no spaces inside. trailing / leading spaces will work, but are frowned upon.
pub fn read_header<R: BufRead>(
    mut inp: R,
) -> Result<HashMap<String, HeadEntry>, DirError> {
    let mut res = HashMap::new();
    let mut linebuf = String::with_capacity(20);
    while inp.read_line(&mut linebuf)? != 0 {
        let parts: Vec<&str> = linebuf.trim().split(",").collect();
        if parts.len() != 3 {
            return Err(DirError::Header(linebuf));
        }
        let ty = TypeSpec::read_short_t(parts[1])?;
        let len = parts[2].parse::<usize>()?;
        res.insert(parts[0].to_string(), HeadEntry { ty, len });
        linebuf.clear();
    }
    Ok(res)
}

pub struct DirHandler;

impl DirStore for DirHandler {
    type MemInfo = HeadEntry;
    type Err = DirError;
    fn read_header<R: BufRead>(
        &self,
        src: R,
    ) -> Result<HashMap<String, Self::MemInfo>, Self::Err> {
        read_header(src)
    }
    fn read_data<R: BufRead>(
        &self,
        mut src: R,
        inf: Self::MemInfo,
    ) -> Result<SingleMem, Self::Err> {
        let mut data = Vec::with_capacity(inf.len);
        let mut linebuf = String::with_capacity(20); // TODO: move outside all?

        // TODO: add a length check
        while src.read_line(&mut linebuf)? != 0 {
            let wo_comment = discard_comment(&linebuf);
            if wo_comment.is_empty() {
                linebuf.clear();
                continue;
            }
            let v = num_ir::numimpl::read_hexstring(
                wo_comment,
                Endian::Little,
                inf.ty.width,
            )?;
            data.push(v);
            linebuf.clear();
        }

        let dimensions = smallvec![data.len()];
        debug_assert_eq!(data.len(), inf.len);
        Ok(SingleMem::new(data, dimensions, inf.ty, Endian::Little))
    }
    fn write_header_part<W: Write>(
        &self,
        inp: &SingleMem,
        mem_name: &str,
        mut dest: W,
    ) -> Result<(), Self::Err> {
        writeln!(dest, "{},{},{}", mem_name, inp.ty(), inp.size())?;
        Ok(())
    }
    fn write_data<W: Write>(
        &self,
        inp: SingleMem,
        _mem_name: String,
        mut dest: W,
    ) -> Result<(), Self::Err> {
        for val in inp.iter_data() {
            writeln!(dest, "{}", val.to_hex_str())?;
        }
        Ok(())
    }
}

// extract a hexstring of given length from the String
// does very basic things to reject commented strings
// TODO: may need to relax 0x restrictions
fn discard_comment(s: &str) -> &str {
    let tr = s.trim();
    let comment_idx = tr.find("//");
    if let Some(idx) = comment_idx {
        let (res, _) = tr.split_at(idx);
        return res;
    }
    tr
}

// TODO: proptest comment discarding
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_discard_comment() {
        assert_eq!(discard_comment("//"), "");
        assert_eq!(discard_comment("// abcde"), "");
        assert_eq!(discard_comment("// abcd // defhqwjewqe"), "");
        assert_eq!(discard_comment("       // abcd // defhqwjewqe"), "       ");
        assert_eq!(discard_comment("abcdef "), "abcdef ");
        assert_eq!(discard_comment("abcdef // foo"), "abcdef ");
        assert_eq!(discard_comment("abcdef // foo bar // baz"), "abcdef ");
        assert_eq!(discard_comment("abcdef//foobar"), "abcdef");
        assert_eq!(discard_comment("abcdef // foo baz"), "abcdef ");
        assert_eq!(discard_comment("abcdef"), "abcdef");
    }
}
