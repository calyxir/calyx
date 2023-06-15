use std::io::prelude::*;
use std::io::{Read, Write};
use std::net::TcpStream;

pub fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:8080")
        .expect("Failed to connect to server");

    // Request 1
    let request1 = r#"Content-Length: 210

{
    "seq": 152,
    "type": "request",
    "command": "initialize",
    "arguments": {
        "adapterId": "0001e357-72c7-4f03-ae8f-c5b54bd8dabf",
        "clientName": "Some Cool Editor"
    }
}
"#;

    stream.write(request1.as_bytes())?;

    let mut response1 = String::new();
    stream.read_to_string(&mut response1)?;
    println!("Received response: {}", response1);

    // Request 2
    let request2 = r#"Content-Length: 119

{
    "seq": 153,
    "type": "request",
    "command": "next",
    "arguments": {
        "threadId": 3
    }
}
"#;

    stream.write(request2.as_bytes())?;

    let mut response2 = String::new();
    stream.read_to_string(&mut response2)?;
    println!("Received response: {}", response2);

    Ok(())
}
