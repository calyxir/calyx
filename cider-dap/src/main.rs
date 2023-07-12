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

/** write func called run_server which takes server type with generic parameters
 * will handle for both single and multi
 *
 * main will instantiate the server with the approporiate read/rite arguments and pass it to run_server
*/
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
impl From<std::io::Error> for MyAdapterError {
    fn from(err: std::io::Error) -> Self {
        MyAdapterError::TcpListenerError(err)
    }
}

fn main() -> Result<(), MyAdapterError> {
    let opts: Opts = argh::from_env();
    println!("{:?}", opts.file);
    let path = opts.file.ok_or(MyAdapterError::MissingFile)?;
    let file = File::open(path).map_err(|_| MyAdapterError::IO)?;
    let adapter = MyAdapter::new(file);

    if opts.is_multi_session {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", opts.port))
            .map_err(MyAdapterError::TcpListenerError)?;
        match listener.accept() {
            Ok((stream, addr)) => {
                println!("Accepted client on: {}", addr);
                let read_stream = BufReader::new(
                    stream
                        .try_clone()
                        .map_err(MyAdapterError::TcpListenerError)?,
                );
                let write_stream = BufWriter::new(stream);
                let server = Server::new(read_stream, write_stream);
                run_server(server, adapter)?;
            }
            Err(e) => return Err(MyAdapterError::TcpListenerError(e)),
        }
    } else {
        let write = BufWriter::new(stdout());
        let read = BufReader::new(stdin());
        let server = Server::new(read, write);
        run_server(server, adapter)?;
    }

    Ok(())
}

fn run_server<R: Read, W: Write>(
    _server: Server<R, W>,
    _adapter: MyAdapter,
) -> AdapterResult<()> {
    todo!()
}
