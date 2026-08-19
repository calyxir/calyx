//! implements the [crate::filerep::DirStore] trait for [DirHandler]

use crate::filerep::*;
use baa::BitVecOps;
use num_ir::memrep::SingleMem;
use num_ir::typing::Endian;
use std::io::{BufRead, Write};

use smallvec::smallvec;

// some use of https://nnethercote.github.io/perf-book/heap-allocations.html#reading-lines-from-a-file throughout. it does actually have a measurable impact.

use std::collections::HashMap;

use num_ir::{short::TryFromShort, typing::TypeSpec};

/// the remainder of what a line within a data header should contain: type and length (number of elements) within a memory.
#[derive(Debug)]
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
    #[error("more entries in a file than expected")]
    BadLen,

    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    ShortT(#[from] num_ir::short::ShortTypeErr),
    #[error(transparent)]
    NumIr(#[from] num_ir::typing::NumParseErr),
}

/// read functionality for a plain text directory header format.
///
/// format: ``name,short_t,len\n``
///
/// no spaces within the string. trailing / leading spaces will work, but are frowned upon.
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

/// handler for the verilator / icarus hex directory format.
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
        /*
            NOTE: this buffer used to be shared amongst all file reads, but isn't now due to structural changes. it isn't too consequential now, but re-sharing it would be nice.
        */
        let mut linebuf = String::with_capacity(20);

        let mut lines_read = 0; // tracked to avoid overflow
        while src.read_line(&mut linebuf)? != 0 {
            if lines_read > inf.len {
                return Err(DirError::BadLen);
            }
            let cleaned_s = discard_comment(&linebuf);
            if cleaned_s.is_empty() {
                // nothing remains after the comment, discard
                linebuf.clear();
                continue;
            }
            let lineval = num_ir::numimpl::read_hexstring(
                cleaned_s,
                Endian::Little,
                inf.ty.width,
            )?;
            data.push(lineval);
            linebuf.clear();
            lines_read += 1;
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
            // NOTE: does not handle endianness
            // NOTE: returns String, this is probably bad for performance
            writeln!(dest, "{}", val.to_hex_str())?;
        }
        Ok(())
    }
}

// extract a hexstring of given length from the String
// does very basic things to reject commented strings
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
    use num_ir::props::*;
    use proptest::prelude::*;

    #[test]
    fn test_discard_comment() {
        assert_eq!(discard_comment("//"), "");
        assert_eq!(discard_comment("// abcde"), "");
        assert_eq!(discard_comment("// abcd // defhqwjewqe"), "");
        assert_eq!(discard_comment("       // abcd // defhqwjewqe"), "");
        assert_eq!(discard_comment("abcdef "), "abcdef");
        assert_eq!(discard_comment("abcdef // foo"), "abcdef ");
        assert_eq!(discard_comment("abcdef // foo bar // baz"), "abcdef ");
        assert_eq!(discard_comment("abcdef//foobar"), "abcdef");
        assert_eq!(discard_comment("abcdef // foo baz"), "abcdef ");
        assert_eq!(discard_comment("abcdef"), "abcdef");
    }

    prop_compose! {
        fn header_entry()(len in 1..usize::MAX,  t in arb_type()) -> HeadEntry{
            HeadEntry{ty: t, len}
        }
    }

    fn mk_header() -> impl Strategy<Value = HashMap<String, HeadEntry>> {
        prop::collection::hash_map("[[:word:]]*", header_entry(), 10)
    }

    proptest! {
    // discarding comment should generally not crash
    #[test]
    fn discard_safety(s in "\\PC*"){
        discard_comment(&s);
    }
    // we don't handle extra "/" well, but for our purposes it should be fine.
    #[test]
    fn prop_discard_comment(before in "[[:word:]]*", after in "[[:word:]]*"){
        let line = format!("{}//{}", before, after);
        prop_assert_eq!(discard_comment(&line), before);

    }

    #[test]
    fn header_roundtrip(hd in mk_header()){
        let mut t: Vec<u8> = Vec::new();
        for (k,v) in hd.iter(){
            writeln!(t, "{},{},{}", k, v.ty, v.len).unwrap();
        }
        let nh = read_header(t.as_slice()).unwrap();
        for k in nh.keys(){
            prop_assert!(hd.contains_key(k));
            let expc_e = hd.get(k).unwrap();
            let got_e = nh.get(k).unwrap();
            prop_assert_eq!(expc_e.len, got_e.len);
            compare_typeclasses(&expc_e.ty,
                &got_e.ty)?;
        }


    }

    }
}
