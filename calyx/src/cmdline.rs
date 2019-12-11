#[macro_use]
extern crate clap;

#[derive(clap)]
#[clap(version = "0.1.0", author = "Samuel Thomas, Kenneth Fang")]
pub struct Opts {
    #[clap(name = "FILE", required = true)]
    file: String,
}
