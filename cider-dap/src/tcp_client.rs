use std::io::{Read, Write};
use std::net::TcpStream;

pub(crate) fn send_request(
    stream: &mut TcpStream,
    request: &str,
) -> std::io::Result<String> {
    let content_length = request.len();

    let formatted_request =
        format!("Content-Length: {}\r\n\r\n{}", content_length, request);

    stream.write_all(formatted_request.as_bytes())?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;

    Ok(response)
}

fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:8080")
        .expect("Failed to connect to server");

    // Request 1
    let request1 = r#"{
        "seq": 1,
        "type": "request",
        "command": "initialize",
        "arguments": {
            "adapterID": "0001e357-72c7-4f03-ae8f-c5b54bd8dabf",
            "clientName": "Some Cool Editor"
        }
    }"#;

    let response1 = send_request(&mut stream, request1)?;
    println!("Received response 1: {}", response1);

    // Request 2
    let request2 = r#"{
        "seq": 2,
        "type": "request",
        "command": "next",
        "arguments": {
            "threadId": 3
        }
    }"#;

    let response2 = send_request(&mut stream, request2)?;
    println!("Received response 2: {}", response2);

    Ok(())
}
