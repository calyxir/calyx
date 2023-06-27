use dap::prelude::*;

pub struct MyAdapter;

pub mod error {
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum MyAdapterError {
        #[error("Unhandled command")]
        UnhandledCommandError,
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
            Command::Initialize(args) => {
                eprintln!("entered initialize handler");
                if let Some(client_name) = args.client_name.as_ref() {
                    eprintln!(
                        "> Client '{client_name}' requested initialization."
                    );
                    Ok(Response::make_success(
                        &request,
                        ResponseBody::Initialize(Some(types::Capabilities {
                            supports_configuration_done_request: Some(true),
                            supports_evaluate_for_hovers: Some(true),
                            ..Default::default()
                        })),
                    ))
                } else {
                    Ok(Response::make_error(&request, "Missing client name"))
                }
            }
            Command::Next(_) => Ok(Response::make_ack(&request).unwrap()),
            _ => Err(MyAdapterError::UnhandledCommandError),
        }
    }
}
