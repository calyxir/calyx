use crate::filerep::*;
use num_ir::typing::TypeSpec;
use num_ir::{memrep::*, typing::Endian};
use smallvec::SmallVec;
use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
};
use struson::reader::{JsonReader, JsonStreamReader};
use struson::writer::{JsonStreamWriter, JsonWriter};

use crate::yxi::*;

pub struct DirData(pub BTreeMap<String, SingleMem>);

impl TryFromIR for DirData {
    fn try_from_ir(inp: FileMems) -> Result<Self, FileFmtErr> {
        let mut res = BTreeMap::new();
        for (k, v) in inp.mems.into_iter() {
            res.insert(k, v);
        }
        Ok(DirData(res))
    }
}

// extract a hexstring of given length from the String
// does very basic things to reject commented strings
fn discard_comment(s: &String) -> &str {
    let comment_idx = s.find("//");
    if let Some(idx) = comment_idx {
        let (res, _) = s.split_at(idx);
        return res;
    }
    &s
}

// TODO: type inference will be forever broken
impl TryToIR for DirData {
    fn try_to_ir(
        self,
        types: &HashMap<String, TypeSpec>,
    ) -> Result<FileMems, FileFmtErr> {
        let mut new_mems = FileMems {
            mems: BTreeMap::new(),
        };

        for (k, v) in self.0.into_iter() {
            // let ty = types.get(&k).unwrap();
            new_mems.mems.insert(k, v);
        }
        Ok(new_mems)
    }
}

impl DirIO for DirData {
    fn read_into_dir(
        src: &std::path::Path,
        ext: String,
    ) -> Result<Self, FileFmtErr> {
        if !src.is_dir() {
            return Err(FileFmtErr::FileSpecific(format!(
                "{:?}: not a directory",
                src
            )));
        }

        let mut header_info: HashMap<String, MemInfo> = HashMap::new();

        let header_file = File::open(src.join(".header"))?;
        let header_r = BufReader::new(header_file);

        let mut sr = JsonStreamReader::new(header_r);
        sr.begin_object()?;
        while sr.has_next()? {
            let k = sr.next_name_owned()?;
            let v: MemInfo = sr.deserialize_next()?;
            header_info.insert(k, v);
        }
        sr.end_object()?;

        let mut res = BTreeMap::new();

        for (mem_name, mi) in header_info {
            let mem_file = BufReader::new(File::open(
                src.join(format!("{}.{}", mem_name, ext)),
            )?);

            let expc_t = TypeSpec::try_from(&mi.format)?;

            let mut data = Vec::new();

            for line in mem_file.lines() {
                let line = line?;
                let line_data = discard_comment(&line);
                let v = expc_t.read_str(line_data, Endian::Little)?;
                data.push(v);
            }

            assert_eq!(data.len(), mi.info.total_size as usize);
            let mut dims_vec = SmallVec::new();
            for i in mi.info.dimension_sizes {
                dims_vec.push(i as usize)
            }
            assert!(dims_vec.len() <= 4);
            res.insert(
                mem_name,
                SingleMem::new(data, dims_vec, expc_t, Endian::Little),
            );
        }

        Ok(DirData(res))
    }
    fn write_out_dir(
        self,
        dest: &std::path::Path,
        ext: String,
    ) -> Result<(), FileFmtErr> {
        if dest.exists() && !dest.is_dir() {
            return Err(FileFmtErr::FileSpecific(format!(
                "{:?}: not a directory",
                dest
            )));
        } else if !dest.exists() {
            std::fs::create_dir(dest)?;
        }

        let mut header_info: HashMap<String, MemInfo> = HashMap::new();

        for (k, v) in &self.0 {
            let file = File::create(dest.join(format!("{}.{}", k, ext)))?;
            let mut writer = BufWriter::new(file);
            for e in v.iter_data() {
                writeln!(writer,);
            }
            header_info.insert()
        }
        let mut header_output = File::create(dest.join(".header"))?;
        let mut header_w = BufWriter::new(header_output);

        let mut sw = JsonStreamWriter::new(header_w);
        sw.begin_object()?;
        for (k, v) in header_info.into_iter() {
            sw.name(&k)?;
            sw.serialize_value(&v)?;
        }
        sw.end_object()?;
        header_w.flush()?;
        Ok(())
    }
}
