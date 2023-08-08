mod adapter;
mod error;

use adapter::MyAdapter;
use error::MyAdapterError;

use dap::prelude::*;
use error::AdapterResult;
use std::fs::File;
use std::io::{stdin, stdout, BufReader, BufWriter, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;

#[derive(argh::FromArgs)]
/// Positional arguments for file path
struct Opts {
    /// input file
    #[argh(positional, from_str_fn(read_path))]
    file: Option<PathBuf>,
    #[argh(switch, long = "tcp")]
    /// runs in tcp mode
    is_multi_session: bool,
    #[argh(option, short = 'p', long = "port", default = "8080")]
    /// port for the TCP server
    port: u16,
}
fn read_path(path: &str) -> Result<PathBuf, String> {
    Ok(PathBuf::from(path))
}

fn main() -> Result<(), MyAdapterError> {
    let opts: Opts = argh::from_env();

    if opts.is_multi_session {
        eprintln!("running multi-session");
        let listener = TcpListener::bind(("127.0.0.1", opts.port))?;
        eprintln!("bound on port: {} ", opts.port);
        let (stream, addr) = listener.accept()?;
        println!("Accepted client on: {}", addr);
        let read_stream = BufReader::new(stream.try_clone()?);
        let write_stream = BufWriter::new(stream);
        let mut server = Server::new(read_stream, write_stream);
        multi_session_init(&mut server)?;
    } else {
        let path = opts.file.ok_or(MyAdapterError::MissingFile)?;
        let file = File::open(path)?;
        let adapter = MyAdapter::new(file);
        eprintln!("running single-session");
        let write = BufWriter::new(stdout());
        let read = BufReader::new(stdin());
        let server = Server::new(read, write);
        // run_server(server, adapter)?;
    }
    eprintln!("exited run_Server");
    Ok(())
}

fn multi_session_init<R, W>(server: &mut Server<R, W>) -> AdapterResult<()>
where
    R: Read,
    W: Write,
{
    // handle the first request (Initialize)
    let req = match server.poll_request()? {
        Some(req) => req,
        None => return Err(MyAdapterError::MissingCommandError),
    };
    match &req.command {
        Command::Initialize(_) => {
            let rsp =
                req.success(ResponseBody::Initialize(types::Capabilities {
                    ..Default::default()
                }));
            server.respond(rsp)?;
            server.send_event(Event::Initialized)?;
        }
        _ => unreachable!(),
    }

    // handle the second request (Launch)
    let req = match server.poll_request()? {
        Some(req) => req,
        None => return Err(MyAdapterError::MissingCommandError),
    };

    let program_path = if let Command::Launch(ref params) = &req.command {
        if let Some(data) = &params.additional_data {
            if let Some(program_path) = data.get("program") {
                eprintln!("Program path: {}", program_path);
                program_path
                    .as_str()
                    .ok_or(MyAdapterError::InvalidPathError)?
            } else {
                return Err(MyAdapterError::MissingFile);
            }
        } else {
            return Err(MyAdapterError::MissingFile);
        }
    } else {
        unreachable!();
    };

    // Open file using the extracted program path
    let file = File::open(program_path)?;

    // Construct the adapter
    let adapter = MyAdapter::new(file);

    // Now, run the server with the adapter
    run_server(server, adapter)
}

fn run_server<R: Read, W: Write>(
    server: &mut Server<R, W>,
    _adapter: MyAdapter,
) -> AdapterResult<()> {
    loop {
        // Start looping here
        let req = match server.poll_request()? {
            Some(req) => req,
            None => return Err(MyAdapterError::MissingCommandError),
        };
        match &req.command {
            Command::Launch(ref params) => {
                let rsp = req.success(ResponseBody::Launch);
                server.respond(rsp)?;
            }
            // Here, you could add a match pattern for a disconnect or exit command
            // to break out of the loop and close the server.
            // Command::Disconnect(_) => break,
            // ...
            unknown_command => {
                eprintln!("Unknown_command: {:?}", unknown_command);
                return Err(MyAdapterError::UnhandledCommandError);
            }
        }
    }
    Ok(())
}
