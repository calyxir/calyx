use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/* pub(crate) fn send_request(
    stream: &mut TcpStream,
    request: &str,
) -> std::io::Result<String> {
    let content_length = request.len();

    let formatted_request =
        format!("Content-Length: {}\r\n\r\n{}", content_length, request);

    println!("{} ", formatted_request);
    stream.write_all(formatted_request.as_bytes())?;
    eprint!("finished writing");
    let mut response = String::new();
    let mut buf = vec![0u8; 1024];
    dbg!(stream.peek(&mut buf).unwrap());

    // Read the response until "\r\n\r\n" is encountered, indicating the end of the headers
    let headers_end = b"\r\n\r\n";
    // let mut headers = Vec::new();
    let reader = BufReader::new(stream);

    for line in reader.lines() {
        let line = line.unwrap();
        println!("Read line: {}", line);

        if line == "\r\n\r\n" {
            println!("Found the sequence. Exiting loop.");
            break;
        }
    }

    Ok(response)
} */
//working?
/* pub(crate) fn send_request(
    stream: &mut TcpStream,
    request: &str,
) -> std::io::Result<String> {
    let content_length = request.len();

    let formatted_request =
        format!("Content-Length: {}\r\n\r\n{}", content_length, request);

    println!("{} ", formatted_request);
    stream.write_all(formatted_request.as_bytes())?;
    eprint!("finished writing");

    let mut response = String::new();
    let mut buf = vec![0u8; 1024];
    dbg!(stream.peek(&mut buf).unwrap());

    // Read the header line (first line)
    let mut reader = BufReader::new(stream);
    let mut header_line = String::new();
    reader.read_line(&mut header_line)?;

    println!("Header line: {}", header_line);

    // Read the empty line (second line)
    let mut empty_line = String::new();
    reader.read_line(&mut empty_line)?;

    println!("Empty line: {}", empty_line);

    // Read the specified bytes of the message
    let mut message_buf = vec![0u8; content_length];
    reader.read_exact(&mut message_buf)?;

    response = String::from_utf8_lossy(&message_buf).to_string();

    Ok(response)
} */
pub(crate) fn send_request(
    stream: &mut TcpStream,
    request: &str,
) -> std::io::Result<String> {
    let content_length = request.len();

    let formatted_request =
        format!("Content-Length: {}\r\n\r\n{}", content_length, request);

    println!("{} ", formatted_request);
    stream.write_all(formatted_request.as_bytes())?;
    eprint!("finished writing");

    let mut response = String::new();
    let mut buf = vec![0u8; 1024];
    dbg!(stream.peek(&mut buf).unwrap());

    // Read the header line (first line)
    let mut reader = BufReader::new(stream);
    let mut header_line = String::new();
    reader.read_line(&mut header_line)?;

    println!("Header line: {}", header_line);

    // Read the empty line (second line)
    let mut empty_line = String::new();
    reader.read_line(&mut empty_line)?;

    println!("Empty line: {}", empty_line);

    // Read the specified bytes of the message
    let mut message_buf = Vec::new();
    reader.read_until(b'\n', &mut message_buf).unwrap();

    response = String::from_utf8_lossy(&message_buf).to_string();

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
    println!("Received response 1: {}", response1);

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
    println!("Received response 2: {}", response2);

    Ok(())
}
