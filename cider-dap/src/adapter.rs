use std::net::TcpListener;

use dap::prelude::*;
use error::MyAdapterError;
pub struct MyAdapter;

pub type AdapterResult<T> = Result<T, MyAdapterError>;
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
