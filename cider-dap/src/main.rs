mod adapter;
mod error;

use adapter::MyAdapter;
use error::MyAdapterError;

use dap::prelude::*;
use error::AdapterResult;
use std::fs::File;
use std::io::{stdin, stdout, BufReader, BufWriter, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;

#[derive(argh::FromArgs)]
/// Positional arguments for file path
struct Opts {
    /// input file
    #[argh(positional, from_str_fn(read_path))]
    file: Option<PathBuf>,
    #[argh(switch, long = "tcp")]
    /// runs in tcp mode
    is_multi_session: bool,
    #[argh(option, short = 'p', long = "port", default = "8080")]
    /// port for the TCP server
    port: u16,
}
fn read_path(path: &str) -> Result<PathBuf, String> {
    Ok(PathBuf::from(path))
}

fn main() -> Result<(), MyAdapterError> {
    let opts: Opts = argh::from_env();
    let path = opts.file.ok_or(MyAdapterError::MissingFile)?;
    let file = File::open(path)?;
    let adapter = MyAdapter::new(file);

    if opts.is_multi_session {
        eprintln!("running multi-session");
        let listener = TcpListener::bind(("127.0.0.1", opts.port))?;
        eprintln!("bound on port: {} ", opts.port);
        let (stream, addr) = listener.accept()?;
        println!("Accepted client on: {}", addr);
        let read_stream = BufReader::new(stream.try_clone()?);
        let write_stream = BufWriter::new(stream);
        let server = Server::new(read_stream, write_stream);

        run_server(server, adapter)?;
    } else {
        eprintln!("running single-session");
        let write = BufWriter::new(stdout());
        let read = BufReader::new(stdin());
        let server = Server::new(read, write);
        run_server(server, adapter)?;
    }
    eprintln!("exited run_Server");
    Ok(())
}

fn run_server<R: Read, W: Write>(
    _server: Server<R, W>,
    _adapter: MyAdapter,
) -> AdapterResult<()> {
    println!("inside run_server");
    Ok(()) //still need to implement this
}
