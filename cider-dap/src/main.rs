mod adapter;
mod error;
use dap::server;

use adapter::MyAdapter;
use error::MyAdapterError;

use dap::prelude::*;
use error::AdapterResult;
use std::fs::File;
use std::io::{stdin, stdout, BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;

/** write func called run_server which takes server type with generic parameters
 * will handle for both single and multi
 *
 * main will instantiate the server with the approporiate read/rite arguments and pass it to run_server
*/
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

fn main() -> AdapterResult<()> {
    let opts: Opts = argh::from_env();
    println!("{:?}", opts.file);
    let path = opts.file.expect("missing file"); //will fix later
    let file = File::open(path).expect("unable to open file"); //will properly address the error later
    let adapter = MyAdapter::new(file);

    if opts.is_multi_session {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", opts.port))
            .expect("binding port failed"); //will fix later

        match listener.accept() {
            Ok((stream, addr)) => {
                println!("Accepted client on: {}", addr);
                let read_stream = BufReader::new(
                    stream.try_clone().expect("failed to clone stream"),
                );
                let write_stream = BufWriter::new(stream);
                let server = Server::new(read_stream, write_stream);
                run_server(server, adapter)?;
            }
            Err(_) => todo!(),
        }
    } else {
        let write = BufWriter::new(stdout());
        let read = BufReader::new(stdin());
        let server = Server::new(read, write);
        run_server(server, adapter)?;
    }

    Ok(())
}

fn run_server<R: Read, W: Write>(
    server: Server<R, W>,
    adapter: MyAdapter,
) -> AdapterResult<()> {
    todo!()
}

/* fn handle_client_stdio(file: PathBuf) -> AdapterResult<()> {
    let f = File::open(file)?;
    let input = BufReader::new(f);
    let output = BufWriter::new(std::io::stdout());
    let mut server = Server::new(input, output);

    let req = match server.poll_request()? {
        Some(req) => req,
        None => return Err(*Box::new(MyAdapterError::MissingCommandError)),
    };
    if let Command::Initialize(_) = req.command {
        let rsp =
            req.success(ResponseBody::Initialize(Some(types::Capabilities {
                ..Default::default()
            })));

        server.respond(rsp)?;

        server.send_event(Event::Initialized)?;
    } else {
        return Err(*Box::new(MyAdapterError::UnhandledCommandError));
    }

    Ok(())
}
fn handle_client_tcp(
    mut stream: TcpStream,
    file: PathBuf,
) -> AdapterResult<()> {
    let f = File::open(file).map_err(|e| {
        MyAdapterError::from(error::MyAdapterError::TcpListenerError(e))
    })?;
    let input = BufReader::new(f);
    let output = BufWriter::new(&mut stream);
    let mut server = Server::new(input, output);

    let req = match server.poll_request()? {
        Some(req) => req,
        None => return Err(MyAdapterError::MissingCommandError.into()),
    };
    if let Command::Initialize(_) = req.command {
        let rsp =
            req.success(ResponseBody::Initialize(Some(types::Capabilities {
                ..Default::default()
            })));

        server.respond(rsp)?;

        server.send_event(Event::Initialized)?;
    } else {
        return Err(MyAdapterError::UnhandledCommandError.into());
    }

    Ok(())
} */
