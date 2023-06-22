use std::io::{BufReader, BufWriter, Write};
use std::net::TcpStream;

use dap::prelude::*;

#[doc = "A modified variant of BasicClient that works well with TcpStreams."]
pub struct TcpClient {
    buff_writer: BufWriter<TcpStream>,
    should_exit: bool,
}

impl TcpClient {
    /// Creates a new `TcpClient` instance with the specified `TcpStream`.
    ///
    /// # Arguments
    ///
    /// * `stream` - The `TcpStream` to use for communication.
    #[doc = "Creates a new TcpClient."]
    pub fn new(stream: TcpStream) -> Self {
        TcpClient {
            buff_writer: BufWriter::new(stream),
            should_exit: false,
        }
    }
    /// Sends a serializable object `s` over the TCP connection.
    ///
    /// # Arguments
    ///
    /// * `s` - The serializable object to send.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with serialization or IO.
    #[doc = "Sends a message to the server."]
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
    /// Sends a response over the TCP connection.
    ///
    /// # Arguments
    ///
    /// * `response` - The response to send.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with sending the response.
    #[doc = "Sends a response to the server."]
    fn respond(&mut self, response: Response) -> dap::client::Result<()> {
        self.send(response)
    }
}

impl Context for TcpClient {
    /// Sends an event over the TCP connection.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to send.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with sending the event.
    #[doc = "Sends an event to the server."]
    fn send_event(&mut self, event: Event) -> dap::client::Result<()> {
        self.send(event)
    }
    /// Sends a reverse request over the TCP connection.
    ///
    /// # Arguments
    ///
    /// * `request` - The reverse request to send.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with sending the reverse request.
    #[doc = "Sends a reverse request to the server."]
    fn send_reverse_request(
        &mut self,
        request: ReverseRequest,
    ) -> dap::client::Result<()> {
        self.send(request)
    }
    /// Requests the client to exit.
    #[doc = "Requests that the client exit."]
    fn request_exit(&mut self) {
        self.should_exit = true;
    }
    /// Cancels the exit request.
    #[doc = "Cancels a request to exit."]
    fn cancel_exit(&mut self) {
        self.should_exit = false;
    }
    /// Returns the current exit state of the client.
    ///
    /// # Returns
    ///
    /// * `true` if the client should exit, `false` otherwise.
    #[doc = "Returns the client's exit state."]
    fn get_exit_state(&self) -> bool {
        self.should_exit
    }
}
