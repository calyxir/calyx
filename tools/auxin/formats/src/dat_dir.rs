use std::collections::HashMap;
use std::io::{BufRead, BufWriter, Read, Write};
use std::path::PathBuf;
use std::{fs::File, io::BufReader};

use crate::cider_dump::*;
use crate::filerep::*;
use baa::{BitVecOps, BitVecValue};
use cider_serde as cs;
use num_ir::memrep::SingleMem;
use num_ir::typing::Endian;

use smallvec::smallvec;

const HEADER_FILENAME: &str = "header";

// in the original cider data converter code, directory I/O was bolted onto the cider datadump format, this is retained.

pub struct DirData {
    pub header: cs::DataHeader,
    pub mems: HashMap<String, Vec<baa::BitVecValue>>,
}

pub struct DirOpts {
    pub dir: PathBuf,
    pub ext: String,
}

impl From<std::io::Error> for FileFmtErr {
    fn from(value: std::io::Error) -> Self {
        Self::FileSpecific(format!("data dir: {}", value))
    }
}

impl TryFromIR for DirOpts {
    fn try_from_ir(self, inp: FileMems) -> Result<(), FileFmtErr> {
        let header_fn = self.dir.join(HEADER_FILENAME);
        if self.dir.exists() && !self.dir.is_dir() {
            return Err(FileFmtErr::FileSpecific(format!(
                "{:?}: not a directory",
                self.dir
            )));
        } else if !self.dir.exists() {
            std::fs::create_dir(&self.dir)?;
        }
        let mut header_info = cs::DataHeader::new("".to_string(), vec![]);

        for (mem_name, mem) in inp.mems {
            let file = File::create(
                self.dir.join(format!("{}.{}", mem_name, self.ext)),
            )?;
            let mut writer = BufWriter::new(file);
            for val in mem.iter_data() {
                write!(writer, "{}\n", val.to_hex_str())?;
            }
            header_info.memories.push(cider_serde::MemoryDeclaration {
                name: mem_name.clone(),
                dimensions: as_cider_dims(&mem),
                format: try_type_to_cider(mem.ty())?,
            });
        }

        let mut header_output = File::create(header_fn)?;
        header_output.write_all(&header_info.serialize()?)?;
        Ok(())
    }
}

impl TryToIR for DirOpts {
    fn try_to_ir(self) -> Result<FileMems, FileFmtErr> {
        if !self.dir.is_dir() {
            return Err(FileFmtErr::FileSpecific(format!(
                "{:?}: not a directory",
                self.dir
            )));
        }

        let header = {
            let mut header_file = File::open(self.dir.join(HEADER_FILENAME))?;
            let mut raw_header = vec![];
            header_file.read_to_end(&mut raw_header)?;

            cs::DataHeader::deserialize(&raw_header)?
        };
        let mut res = FileMems::default();

        for mem_dec in &header.memories {
            let mut data = Vec::with_capacity(mem_dec.size());
            let mem_file = BufReader::new(File::open(
                self.dir.join(format!("{}.{}", mem_dec.name, self.ext)),
            )?);

            for line in mem_file.lines() {
                let line = line?;
                let wo_comment = discard_comment(&line);
                data.push(BitVecValue::from_hex_str(wo_comment)?);
            }

            let dimensions = smallvec![data.len()];

            // assert_eq!(data.len() - starting_len, mem_dec.byte_count());
            res.mems.insert(
                mem_dec.name.clone(),
                SingleMem::new(
                    data,
                    dimensions,
                    try_type_from_cider(&mem_dec.format)?,
                    Endian::Little,
                ),
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
