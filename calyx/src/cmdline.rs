use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS")
)]
pub struct Opts {
    // name of path where futil program lives
    #[structopt(required = true, parse(from_os_str))]
    pub file: PathBuf,

    // name of the component in <file>
    #[structopt(short, long, default_value = "main")]
    pub component: String,

    // optional argument that optionally takes a value
    #[structopt(short = "s", long = "show-struct")]
    pub visualize_structure: Option<Option<PathBuf>>,

    // optional argument that optionally takes a path to output png
    #[structopt(long = "show-fsm")]
    pub visualize_fsm: Option<Option<PathBuf>>,

    // library paths
    #[structopt(long, short)]
    pub libraries: Vec<PathBuf>,
}

// ================== Helper Functions ======================= //

pub struct Writer {
    file: Option<File>,
}

impl std::fmt::Write for Writer {
    fn write_str(&mut self, msg: &str) -> Result<(), std::fmt::Error> {
        match &mut self.file {
            None => print!("{}", msg),
            Some(f) => write!(f, "{}", msg).unwrap(),
        }
        Ok(())
    }
}

pub fn path_write(
    path: &Option<PathBuf>,
    write_fn: Box<dyn FnOnce(&mut Writer) -> Result<(), std::fmt::Error>>,
) {
    match path {
        None => write_fn(&mut Writer { file: None }).unwrap(),
        Some(p) => write_fn(&mut Writer {
            file: Some(File::create(p.as_path()).unwrap()),
        })
        .unwrap(),
    }
}
