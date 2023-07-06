mod adapter;
mod client;
mod error;

use adapter::MyAdapter;
use client::TcpClient;
use error::MyAdapterError;

use dap::prelude::*;
use error::AdapterResult;
use std::io::BufReader;
use std::net::{TcpListener, TcpStream};
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

fn main() -> AdapterResult<()> {
    let opts: Opts = argh::from_env();
    println!("{:?}", opts.file);
    let path = match opts.file {
        Some(p) => Ok(p),
        None => Err(MyAdapterError::MissingFile),
    }?;
    /* let file = opts.file.unwrap_or_else(|| {
        eprintln!("Missing input file argument");
        std::process::exit(1);
    });*/

    /*  let listener = TcpListener::bind("127.0.0.1:8080")?;

       match listener.accept() {
           Ok((stream, addr)) => {
               println!("Accepted client on: {}", addr);
               handle_client(stream, file)?;
           }
           Err(_) => todo!(),
       }
    */
    Ok(())
}

fn handle_client(mut stream: TcpStream, file: PathBuf) -> std::io::Result<()> {
    // Create a TcpClient instance with the TCP stream
    let client = TcpClient::new(stream.try_clone()?);

    // Create a Server instance with an appropriate adapter and the client
    let mut server = Server::new(MyAdapter, client);

    // Create a BufReader from the TcpStream
    let mut reader = BufReader::new(&mut stream);

    match server.run(&mut reader) {
        Ok(()) => println!("Request handled successfully"),
        Err(err) => {
            eprintln!("Error handling request: {:?}", err);
        }
    }

    Ok(())
}
