use owo_colors::OwoColorize;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

pub(crate) fn send_request(
    stream: &mut TcpStream,
    request: &str,
) -> std::io::Result<String> {
    let content_length = request.len();

    let formatted_request =
        format!("Content-Length: {}\r\n\r\n{}", content_length, request);

    println!("{} ", formatted_request.magenta().bold());
    stream.write_all(formatted_request.as_bytes())?;

    let mut buf = vec![0u8; 1024];
    dbg!(stream.peek(&mut buf).unwrap());

    // Read the header line (first line)
    let mut reader = BufReader::new(stream);
    let mut header_line = String::new();
    reader.read_line(&mut header_line)?;

    println!("Header line: {}", header_line.green().bold());

    // Read the empty line (second line)
    let mut empty_line = String::new();
    reader.read_line(&mut empty_line)?;

    // Read the specified bytes of the message
    let mut message_buf = Vec::new();
    reader.read_until(b'\n', &mut message_buf).unwrap();

    let response = String::from_utf8_lossy(&message_buf).to_string();
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
            "adapterId": "0001e357-72c7-4f03-ae8f-c5b54bd8dabf",
            "clientName": "Some Cool Editor"
        }}
"#;

    let response1 = send_request(&mut stream, request1)?;
    println!("Received response 1: {}", response1.blue().bold().italic());

    // Request 2
    let request2 = r#"{
        "seq": 2,
        "type": "request",
        "command": "next",
        "arguments": {
            "threadId": 3
        }}
"#;

    let response2 = send_request(&mut stream, request2)?;
    println!("Received response 2: {}", response2.blue().bold().italic());

    Ok(())
}
