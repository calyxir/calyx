use crate::filerep::*;
use baa::{BitVecOps, BitVecValue};
use num_ir::memrep::SingleMem;
use num_ir::typing::Endian;
use std::io::{BufRead, BufWriter, Read, Write};
use std::{fs::File, io::BufReader};

use smallvec::smallvec;

const HEADER_FILENAME: &str = "header";

// some use of https://nnethercote.github.io/perf-book/heap-allocations.html#reading-lines-from-a-file throughout. it does actually have a measurable impact.

use std::collections::HashMap;

use num_ir::{short::TryFromShort, typing::TypeSpec};

use crate::filerep::FileFmtErr;

pub struct HeadEntry {
    pub ty: TypeSpec,
    pub len: usize,
}

/// read functionality for a plain text directory header format.
///
/// format: ``name,short_t,len\n``
///
/// no spaces inside. trailing / leading spaces will work, but are frowned upon.
pub fn read_header<R: Read>(
    mut inp: BufReader<R>,
) -> Result<HashMap<String, HeadEntry>, FileFmtErr> {
    let mut res = HashMap::new();
    let mut linebuf = String::with_capacity(20);
    while inp.read_line(&mut linebuf)? != 0 {
        let parts: Vec<&str> = linebuf.trim().split(",").collect();
        if parts.len() != 3 {
            return Err(FileFmtErr::FileSpecific("bad format".to_string()));
        }
        let ty = TypeSpec::read_short_t(parts[1])
            .map_err(|e| FileFmtErr::FileSpecific(e.to_string()))?;
        let len = parts[2]
            .parse::<usize>()
            .map_err(|e| FileFmtErr::FileSpecific(e.to_string()))?;
        res.insert(parts[0].to_string(), HeadEntry { ty, len });
        linebuf.clear();
    }
    Ok(res)
}

impl From<std::io::Error> for FileFmtErr {
    fn from(value: std::io::Error) -> Self {
        Self::FileSpecific(format!("data dir: {}", value))
    }
}

pub struct DirHandler;

impl TryWriteThrough<DirSink> for DirHandler {
    fn try_write(
        &self,
        dest: DirSink,
        inp: FileMems,
    ) -> Result<(), FileFmtErr> {
        let header_fn = dest.path.join(HEADER_FILENAME);
        dest.is_dir()?;
        if !dest.path.exists() {
            std::fs::create_dir(&dest.path)?;
        }
        let header_output = File::create(header_fn)?;
        let mut header_w = BufWriter::new(header_output);

        // NOTE: there's probably efficiencies to be searched for in here
        for (mem_name, mem) in inp.mems.into_iter() {
            writeln!(header_w, "{},{},{}", mem_name, mem.ty(), mem.size())?;

            // write memory contents
            let file = File::create(
                dest.path.join(format!("{}.{}", mem_name, dest.ext)),
            )?;
            let mut writer = BufWriter::new(file);
            for val in mem.iter_data() {
                writeln!(writer, "{}", val.to_hex_str())?;
            }
        }

        Ok(())
    }
}

impl TryReadFrom<DirSink> for DirHandler {
    fn try_read(&self, src: DirSink) -> Result<FileMems, FileFmtErr> {
        if !src.path.is_dir() {
            return Err(FileFmtErr::FileSpecific(format!(
                "{:?}: not a directory",
                src.path
            )));
        }

        let mut header = {
            let header_file = File::open(src.path.join(HEADER_FILENAME))?;
            let header_r = BufReader::new(header_file);
            read_header(header_r)?
        };
        let mut res = FileMems::default();

        // owned buffer for file reads, because lines() returns a vector of string :(
        let mut linebuf = String::with_capacity(20);

        for (mem_name, info) in header.drain() {
            let mut data = Vec::with_capacity(info.len);
            let mut mem_file = BufReader::new(File::open(
                src.path.join(format!("{}.{}", mem_name, src.ext)),
            )?);

            // TODO: add a length check
            while mem_file.read_line(&mut linebuf)? != 0 {
                let cleaned = linebuf.trim();
                let wo_comment = discard_comment(cleaned);
                if wo_comment.is_empty() {
                    linebuf.clear();
                    continue;
                }
                let v = BitVecValue::from_str_radix(
                    wo_comment,
                    16,
                    info.ty.width as u32,
                )?;
                data.push(v);
                linebuf.clear();
            }

            let dimensions = smallvec![data.len()];
            debug_assert_eq!(data.len(), info.len);

            res.mems.insert(
                mem_name.clone(),
                SingleMem::new(data, dimensions, info.ty, Endian::Little),
            );
        }

        Ok(res)
    }
}

impl From<baa::ParseIntError> for FileFmtErr {
    fn from(value: baa::ParseIntError) -> Self {
        FileFmtErr::BadVal(num_ir::typing::NumParseErr::Baa(
            "".to_string(),
            value,
        ))
    }
}

// extract a hexstring of given length from the String
// does very basic things to reject commented strings
// TODO: needs to relax 0x restrictions
fn discard_comment(s: &str) -> &str {
    let comment_idx = s.find("//");
    if let Some(idx) = comment_idx {
        let (res, _) = s.split_at(idx);
        return res;
    }
    &s
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
