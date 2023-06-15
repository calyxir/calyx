use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

mod tcp_client;

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    println!("Handling client connection...");
    let mut buffer = [0; 1024];
    stream.read(&mut buffer)?; // Read the request message from the client

    // Process the received message and send appropriate responses
    let response = match String::from_utf8_lossy(&buffer).as_ref() {
        "{\"seq\": 152, \"type\": \"request\", \"command\": \"initialize\", \"arguments\": {\"adapterId\": \"0001e357-72c7-4f03-ae8f-c5b54bd8dabf\", \"clientName\": \"Some Cool Editor\"}}" => {
            "{\"seq\": 152, \"type\": \"response\", \"command\": \"initialize\", \"success\": true}"
        }
        "{\"seq\": 153, \"type\": \"request\", \"command\": \"next\", \"arguments\": {\"threadId\": 3}}" => {
            "{\"seq\": 153, \"type\": \"response\", \"command\": \"next\", \"success\": true}"
        }
        _ => "{\"seq\": 0, \"type\": \"response\", \"command\": \"unknown\", \"success\": false}",
    };

    stream.write_all(response.as_bytes())?; // Send the response back to the client
    stream.flush()?; // Flush the stream to ensure data is sent
    Ok(())
}
fn main() {
    let listener =
        TcpListener::bind("127.0.0.1:8080").expect("Failed to bind address");
    listener
        .set_nonblocking(false)
        .expect("Failed to set non-blocking mode");

    println!("DAP server listening on port 8080...");

    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                println!("Handling client connection...");
                handle_client(stream);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No incoming connections yet, handle other tasks or sleep
                // to avoid busy-waiting
                std::thread::sleep(Duration::from_millis(100));
                println!("Err");
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}
