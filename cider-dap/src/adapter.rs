use std::io::{self, Write};

use dap::prelude::*;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyAdapterError {
    #[error("Unhandled command")]
    UnhandledCommandError,
    // Add more error variants as needed
    #[error("Unable to parse the file: {0}")]
    InvalidFile(String),
    #[error("Missing Required file")]
    MissingFile,
    #[error("Issues with IO")]
    IO,
    #[error("Issues with TCPListener")]
    TcpListenerError(std::io::Error),
}

pub type AdapterResult<T> = Result<T, MyAdapterError>;

pub struct MyAdapter;

impl Adapter for MyAdapter {
    type Error = MyAdapterError;

    fn accept(
        &mut self,
        request: Request,
        _ctx: &mut dyn Context,
    ) -> Result<Response, Self::Error> {
        eprintln!("Accept {:#?}\n", request.command);

        match &request.command {
            _ => {
                // Handle the command generically
                Ok(Response::make_ack(&request).unwrap())
            }
        }
    }
}
