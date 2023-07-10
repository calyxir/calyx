mod adapter;
mod client;
mod error;

use adapter::MyAdapter;
use client::TcpClient;
use error::MyAdapterError;

use dap::prelude::*;
use std::convert::From;
use std::io::{stdin, stdout, BufReader, Read, Write};
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
    #[argh(option, short = 'p', default = "8080")]
    /// sets the port for the TCP server
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
    let path = match opts.file {
        Some(p) => Ok(p),
        None => Err(MyAdapterError::MissingFile),
    }?;

    if opts.is_multi_session {
        // TCP mode
        let listener = TcpListener::bind(format!("127.0.0.1:{}", opts.port))
            .map_err(|err| MyAdapterError::from(err))?;
        match listener.accept() {
            Ok((stream, addr)) => {
                println!("Accepted client on: {}", addr);
                handle_client(stream, path)?;
            }
            Err(_) => todo!(),
        }
    } else {
        // stdin/stdout mode
        let mut buffer = Vec::new();
        stdin().read_to_end(&mut buffer)?;

        let input = String::from_utf8_lossy(&buffer).to_string();

        let output = if input.trim().to_lowercase() == "tcp" {
            let stream = TcpStream::connect("127.0.0.1:8080")
                .expect("Failed to connect to server");
            handle_client(stream, path)?;
            "TCP mode activated".to_string()
        } else {
            format!("Received input: {}", input)
        };

        stdout().write_all(output.as_bytes())?;
    }

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
