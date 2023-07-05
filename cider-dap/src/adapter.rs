use std::io::{self, Write};

use dap::prelude::*;

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
        let mut stdout = io::stdout();

        match &request.command {
            Command::Initialize(args) => {
                eprintln!("entered initialize handler");
                if let Some(client_name) = args.client_name.as_ref() {
                    writeln!(
                        stdout,
                        "> Client '{}' requested initialization.",
                        client_name
                    )
                    .map_err(|err| MyAdapterError::from(err))?;
                    stdout.flush().map_err(MyAdapterError::from)?;
                    Ok(Response::make_success(
                        &request,
                        ResponseBody::Initialize(Some(types::Capabilities {
                            supports_configuration_done_request: Some(true),
                            supports_evaluate_for_hovers: Some(true),
                            ..Default::default()
                        })),
                    ))
                } else {
                    writeln!(stdout, "Missing client name")
                        .map_err(MyAdapterError::from)?;
                    stdout.flush().map_err(MyAdapterError::from)?;
                    Ok(Response::make_error(&request, "Missing client name"))
                }
            }
            Command::Next(_) => {
                writeln!(stdout, "Next command received")
                    .map_err(MyAdapterError::from)?;
                stdout.flush().map_err(MyAdapterError::from)?;
                Ok(Response::make_ack(&request).unwrap())
            }
            _ => Err(MyAdapterError::UnhandledCommandError),
        }
    }
}
