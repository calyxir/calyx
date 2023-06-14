use std::fs::File;
use std::io::{BufReader, BufWriter};

use thiserror::Error;

use dap::prelude::*;

#[derive(Error, Debug)]
enum MyAdapterError {
    #[error("Unhandled command")]
    UnhandledCommandError,
}

struct MyAdapter;

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

type DynResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> DynResult<()> {
    let adapter = MyAdapter {};
    let client = BasicClient::new(BufWriter::new(std::io::stdout()));
    let mut server = Server::new(adapter, client);

    let f = File::open("testinput.txt")?;
    let mut reader = BufReader::new(f);

    server.run(&mut reader)?;
    Ok(())
}
