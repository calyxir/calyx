use std::io::{BufRead, BufWriter, Read, Write};
use std::{fs::File, io::BufReader, path::PathBuf};

use crate::dat_parser::*;
use crate::filerep::*;
use cider::serialization as cs;

const DAT_EXTENSION: &str = "dat";

const HEADER_FILENAME: &str = "header";

impl From<std::io::Error> for FileFmtErr {
    fn from(value: std::io::Error) -> Self {
        Self::from(value.to_string())
    }
}

// in the original cider data converter code, directory I/O was bolted onto the cider datadump format, this is retained.

impl DirIO for cs::DataDump {
    fn read_into_dir(src: PathBuf) -> Result<Self, FileFmtErr> {
        if !src.is_dir() {
            return Err(FileFmtErr::from("not a directory"));
        }

        let header = {
            let mut header_file = File::open(src.join(HEADER_FILENAME))?;
            let mut raw_header = vec![];
            header_file.read_to_end(&mut raw_header)?;

            cs::DataHeader::deserialize(&raw_header)?
        };
        let mut data: Vec<u8> = vec![];

        for mem_dec in &header.memories {
            let starting_len = data.len();
            let mem_file = BufReader::new(File::open(
                src.join(format!("{}.{}", mem_dec.name, DAT_EXTENSION)),
            )?);

            for line in mem_file.lines() {
                let line = line?;
                if let Some(line_data) = unwrap_line_or_comment(&line) {
                    assert!(
                        line_data.len() <= mem_dec.bytes_per_entry() as usize,
                        "line data too long"
                    );

                    let padding =
                        (mem_dec.bytes_per_entry() as usize) - line_data.len();

                    data.extend(line_data.into_iter().rev());
                    data.extend(std::iter::repeat_n(0u8, padding))
                }
            }

            assert_eq!(data.len() - starting_len, mem_dec.byte_count());
        }

        Ok(cs::DataDump { header, data })
    }
    fn write_out_dir(&self, dest: PathBuf) -> Result<(), FileFmtErr> {
        if dest.exists() && !dest.is_dir() {
            return Err(FileFmtErr::from("not a directory"));
        } else if !dest.exists() {
            std::fs::create_dir(&dest)?;
        }

        let mut header_output = File::create(dest.join(HEADER_FILENAME))?;
        header_output.write_all(&self.header.serialize()?)?;

        for memory in &self.header.memories {
            let file = File::create(
                dest.join(format!("{}.{}", memory.name, DAT_EXTENSION)),
            )?;
            let mut writer = BufWriter::new(file);
            for bytes in self
                .get_data(&memory.name)
                .unwrap()
                .chunks_exact(memory.bytes_per_entry() as usize)
            {
                // data file seems to expect lsb on the right
                // for the moment electing to print out every byte
                // and do so with two hex digits per byte rather
                // than truncating leading zeroes. No need to do
                // anything fancy here.
                for byte in bytes.iter().rev() {
                    write!(writer, "{byte:02X}")?;
                }

                writeln!(writer)?;
            }
        }
        Ok(())
    }
}
