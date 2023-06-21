use std::io::{BufReader, BufWriter, Write};
use std::net::TcpStream;

use dap::prelude::*;

pub struct TcpClient {
    buff_writer: BufWriter<TcpStream>,
    should_exit: bool,
}

impl TcpClient {
    pub fn new(stream: TcpStream) -> Self {
        TcpClient {
            buff_writer: BufWriter::new(stream),
            should_exit: false,
        }
    }
    fn send<S: serde::Serialize>(&mut self, s: S) -> dap::client::Result<()> {
        let json = serde_json::to_string(&s)
            .map_err(ClientError::SerializationError)?;
        write!(self.buff_writer, "Content-Length: {}\r\n\r\n", json.len())
            .map_err(ClientError::IoError)?;
        write!(self.buff_writer, "{}\r\n", json)
            .map_err(ClientError::IoError)?;
        self.buff_writer.flush().map_err(ClientError::IoError)?;
        Ok(())
    }
}

impl Client for TcpClient {
    fn respond(&mut self, response: Response) -> dap::client::Result<()> {
        self.send(response)
    }
}

impl Context for TcpClient {
    fn send_event(&mut self, event: Event) -> dap::client::Result<()> {
        self.send(event)
    }

    fn send_reverse_request(
        &mut self,
        request: ReverseRequest,
    ) -> dap::client::Result<()> {
        self.send(request)
    }

    fn request_exit(&mut self) {
        self.should_exit = true;
    }

    fn cancel_exit(&mut self) {
        self.should_exit = false;
    }

    fn get_exit_state(&self) -> bool {
        self.should_exit
    }
}
