use dap::prelude::*;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream};
use thiserror::Error;
mod tcp_client;

#[derive(Error, Debug)]
enum MyAdapterError {
    #[error("Unhandled command")]
    UnhandledCommandError,
}

struct MyAdapter;

impl Adapter for MyAdapter {
    type Error = MyAdapterError;

    fn accept(
        &mut self,
        request: Request,
        _ctx: &mut dyn Context,
    ) -> Result<Response, Self::Error> {
        eprintln!("Accept {:#?}\n", request.command);

        match &request.command {
            Command::Initialize(args) => {
                if let Some(client_name) = args.client_name.as_ref() {
                    eprintln!(
                        "> Client '{client_name}' requested initialization."
                    );
                    Ok(Response::make_success(
                        &request,
                        ResponseBody::Initialize(Some(types::Capabilities {
                            supports_configuration_done_request: Some(true),
                            supports_evaluate_for_hovers: Some(true),
                            ..Default::default()
                        })),
                    ))
                } else {
                    Ok(Response::make_error(&request, "Missing client name"))
                }
            }
            Command::Next(_) => Ok(Response::make_ack(&request).unwrap()),
            _ => Err(MyAdapterError::UnhandledCommandError),
        }
    }
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;

    for stream in listener.incoming() {
        let stream = stream?;
        handle_client(stream)?;
    }

    Ok(())
}

struct CloneStreams {
    client_stream: BufReader<TcpStream>,
    dap_stream: TcpStream,
}

impl CloneStreams {
    fn new(client_stream: TcpStream, dap_stream: TcpStream) -> Self {
        Self {
            client_stream: BufReader::new(client_stream),
            dap_stream,
        }
    }
}

impl Read for CloneStreams {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.client_stream.read(buf)
    }
}

impl Write for CloneStreams {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.dap_stream.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.dap_stream.flush()
    }
}

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    println!("Handling client connection...");
    let mut buffer = [0; 1024];

    stream.read(&mut buffer)?; // Read the request message from the client.

    // Create CloneStreams and pass it to handle_dap_rs
    let dap_stream = TcpStream::connect("127.0.0.1:8080")?;
    let clone_streams = CloneStreams::new(stream.try_clone()?, dap_stream);
    handle_dap_rs(clone_streams)?;

    // Create a BasicClient instance with the TCP stream
    let client = BasicClient::new(stream.try_clone()?);

    // Create a Server instance with an appropriate adapter and the client
    let mut server = Server::new(MyAdapter, client);

    // Create a BufReader from the TcpStream
    let mut reader = BufReader::new(&mut stream);

    // Run the server to handle the request and generate a response
    match server.run(&mut reader) {
        Ok(()) => println!("Request handled successfully"),
        Err(err) => eprintln!("Error handling request: {:?}", err),
    }

    Ok(())
}

fn handle_dap_rs(mut streams: CloneStreams) -> std::io::Result<()> {
    // Read from `streams` and send data to `dap-rs` (server.rs).
    // needs change?
    // Example usage:
    let mut buffer = [0; 1024];
    streams.read(&mut buffer)?;
    streams.write_all(&buffer)?;

    Ok(())
}
