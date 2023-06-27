use dap::prelude::*;
use std::io::BufReader;
use std::net::{TcpListener, TcpStream};

mod client;
use client::TcpClient;
mod adapter;
use adapter::MyAdapter;

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

    match server.run(&mut reader) {
        Ok(()) => println!("Request handled successfully"),
        Err(err) => {
            eprintln!("Error handling request: {:?}", err);
        }
    }

    Ok(())
}
