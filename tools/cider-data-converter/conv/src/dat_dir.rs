use std::io::{BufRead, BufWriter, Read, Write};
use std::{fs::File, io::BufReader, path::PathBuf};

use crate::dat_parser::*;
use crate::filerep::*;
use cider::serialization as cs;

pub struct DataDir {
    base_path: PathBuf,
    file_extension: String,
    output_path: PathBuf,
}

const DAT_EXTENSION: &str = "dat";

const HEADER_FILENAME: &str = "header";

impl DataDir {
    fn load(&self) -> Result<cs::DataDump, FileFmtErr> {
        if self.base_path.is_dir() {
            // we are converting from a dat directory rather than a
            // dump

            let header = {
                let mut header_file =
                    File::open(self.base_path.join(HEADER_FILENAME))?;
                let mut raw_header = vec![];
                header_file.read_to_end(&mut raw_header)?;

                cs::DataHeader::deserialize(&raw_header)?
            };

            let mut data: Vec<u8> = vec![];

            for mem_dec in &header.memories {
                let starting_len = data.len();
                let mem_file =
                    BufReader::new(File::open(self.base_path.join(format!(
                        "{}.{}",
                        mem_dec.name, self.file_extension
                    )))?);

                for line in mem_file.lines() {
                    let line = line?;
                    if let Some(line_data) = unwrap_line_or_comment(&line) {
                        assert!(
                            line_data.len()
                                <= mem_dec.bytes_per_entry() as usize,
                            "line data too long"
                        );

                        let padding = (mem_dec.bytes_per_entry() as usize)
                            - line_data.len();

                        data.extend(line_data.into_iter().rev());
                        data.extend(std::iter::repeat_n(0u8, padding))
                    }
                }

                assert_eq!(data.len() - starting_len, mem_dec.byte_count());
            }

            Ok(cs::DataDump { header, data })
        } else {
            Err(String::from("not directory"))
        }
    }

    fn store(&self, data: cs::DataDump) -> Result<(), FileFmtErr> {
        if self.output_path.exists() && !self.output_path.is_dir() {
            return Err(String::from("outpath not dir"));
        } else if !self.output_path.exists() {
            std::fs::create_dir(&self.output_path)?;
        }

        let mut header_output =
            File::create(self.output_path.join(HEADER_FILENAME))?;
        header_output.write_all(&data.header.serialize()?)?;

        for memory in &data.header.memories {
            let file = File::create(
                path.join(format!("{}.{}", memory.name, self.file_extension)),
            )?;
            let mut writer = BufWriter::new(file);
            for bytes in data
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
