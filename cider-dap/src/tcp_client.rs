use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

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

    /*  loop {
           println!("reached loop");
           let bytes_read = match stream.read(&mut buf) {
               Ok(0) => break, // Reached the end of the stream
               Ok(n) => n,
               Err(e) => {
                   println!("Error reading from stream: {}", e);
                   break;
               }
           };
           println!("finished read");
           headers.extend_from_slice(&buf[..bytes_read]);

           let headers_str = String::from_utf8_lossy(&headers);
           if headers_str.contains("\r\n\r\n") {
               break;
           }
       }

       // Print the received response headers
       let headers_str = String::from_utf8_lossy(&headers);
       println!("Response Headers:\n{}", headers_str);

       // Parse the Content-Length header to determine the response body size
       let content_length = headers_str
           .lines()
           .find(|line| line.starts_with("Content-Length:"))
           .and_then(|line| line.split(":").nth(1))
           .and_then(|length| length.trim().parse::<usize>().ok())
           .unwrap_or(0);

       // Read the response body based on the content length
       let mut response_body = vec![0u8; content_length];
       stream.read_exact(&mut response_body)?;

       response.push_str(&String::from_utf8_lossy(&response_body));

       // Print the received response body
       println!("Response Body:\n{}", response);
    */
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
