use std::io::{self, Write};

use dap::prelude::*;
use error::MyAdapterError;
pub struct MyAdapter;

pub mod error {
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum MyAdapterError {
        #[error("Unhandled command")]
        UnhandledCommandError,
        #[error(transparent)]
        Io(#[from] std::io::Error),
        // Add more error variants as needed
    }
}

use error::MyAdapterError;

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
