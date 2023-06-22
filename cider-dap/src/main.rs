use dap::prelude::*;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use thiserror::Error;

mod client;
use client::TcpClient;

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
                eprintln!("entered intialize handler");
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

    match listener.accept() {
        Ok((stream, addr)) => {
            println!("Accepted client on: {} ", addr);
            handle_client(stream)?;
        }
        Err(_) => todo!(),
    }

    Ok(())
}

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    // Create a TcpClient instance with the TCP stream
    let client = TcpClient::new(stream.try_clone()?);

    // Create a Server instance with an appropriate adapter and the client
    let mut server = Server::new(MyAdapter, client);

    // Create a BufReader from the TcpStream
    let mut reader = BufReader::new(&mut stream);

    loop {
        // Run the server to handle the request and generate a response
        match server.run(&mut reader) {
            Ok(()) => println!("Request handled successfully"),
            Err(err) => {
                eprintln!("Error handling request: {:?}", err);
                break; // Exit the loop on error
            }
        }

        // Check if there are more requests to process
        let mut buf = [0; 1];
        match reader.read(&mut buf) {
            Ok(0) => break,    // No more data to read, exit the loop
            Ok(_) => continue, // Continue to handle the next request
            Err(err) => {
                eprintln!("Error reading from stream: {:?}", err);
                break; // Exit the loop on error
            }
        }
    }

    println!("Additional code after handling requests");

    Ok(())
}
