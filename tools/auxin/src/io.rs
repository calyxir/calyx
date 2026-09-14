use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

use conv_formats::filerep::*;

#[derive(Debug, thiserror::Error)]
pub(crate) enum AuxinError {
    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("typing op: {0}")]
    OpError(#[from] num_ir::typing::OpError),

    #[error("Failed to parse \"to\" argument: {0}")]
    ToArgErr(String),

    #[error("internal: {0}")]
    InternalErr(String),

    #[error("Unknown input format. Specify manually?")]
    UnknownIn,

    #[error(
        "Unable to guess the conversion target. Please specify the target using the \"--to\" argument"
    )]
    UnknownTarget,

    #[error(
        "Missing output path. This is required for the \"to dat\" conversion"
    )]
    MissingDatOutputPath,

    #[error("destination path is already file")]
    BadDirPath,
}

const HEADER_FILENAME: &str = "header";

fn wrap_specific_err<E: std::error::Error>(
    err: E,
    slug: &'static str,
) -> AuxinError {
    AuxinError::InternalErr(format!("{}: {}", slug, err))
}

pub(crate) fn file_read<T: FileStore>(
    through: &T,
    path: &Option<PathBuf>,
    slug: &'static str,
) -> Result<MemsMap, AuxinError> {
    if let Some(specific) = path {
        let filebuf = File::open(specific).map(BufReader::new)?;
        through.read_filelike(filebuf)
    } else {
        let stdin_l = std::io::stdin().lock();
        through.read_stream(stdin_l)
    }
    .map_err(|e| wrap_specific_err(e, slug))
}

pub(crate) fn file_write<T: FileStore>(
    through: &T,
    inp: MemsMap,
    path: &Option<PathBuf>,
    slug: &'static str,
) -> Result<(), AuxinError> {
    if let Some(specific) = path {
        let filebuf = File::create(specific).map(BufWriter::new)?;
        through.write(inp, filebuf)
    } else {
        through.write(inp, std::io::stdout())
    }
    .map_err(|e| wrap_specific_err(e, slug))
}

fn is_dir(p: &Path) -> Result<(), AuxinError> {
    if p.exists() && !p.is_dir() {
        return Err(AuxinError::BadDirPath);
    }
    Ok(())
}

pub(crate) fn dir_read(
    through: &conv_formats::dat_dir::DirHandler,
    path: &Path,
    ext: String,
) -> Result<MemsMap, AuxinError> {
    is_dir(path)?;
    let mut header = {
        let header_file = File::open(path.join(HEADER_FILENAME))?;
        let header_r = BufReader::new(header_file);
        through
            .read_header(header_r)
            .map_err(|e| wrap_specific_err(e, "dat_dir"))
    }?;
    let mut res = MemsMap::default();

    for (mem_name, info) in header.drain() {
        let mem_file = BufReader::new(File::open(
            path.join(format!("{}.{}", mem_name, ext)),
        )?);
        let mem_contents = through
            .read_data(mem_file, info)
            .map_err(|e| wrap_specific_err(e, "dat_dir"))?;

        res.mems.insert(mem_name.clone(), mem_contents);
    }

    Ok(res)
}

pub(crate) fn dir_write(
    through: &conv_formats::dat_dir::DirHandler,
    inp: MemsMap,
    path: &Path,
    ext: String,
) -> Result<(), AuxinError> {
    is_dir(path)?;
    if !path.exists() {
        std::fs::create_dir(path)?;
    }
    let header_fn = path.join(HEADER_FILENAME);
    let header_output = File::create(header_fn)?;
    let mut header_w = BufWriter::new(header_output);

    for (mem_name, mem) in inp.mems.into_iter() {
        through
            .write_header_part(&mem, &mem_name, &mut header_w)
            .map_err(|e| wrap_specific_err(e, "dat_dir"))?;

        // write memory contents
        let file = File::create(path.join(format!("{}.{}", mem_name, ext)))?;
        let file_w = BufWriter::new(file);
        through
            .write_data(mem, mem_name, file_w)
            .map_err(|e| wrap_specific_err(e, "dat_dir"))?;
    }

    Ok(())
}
