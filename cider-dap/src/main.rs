mod adapter;
mod error;

use adapter::MyAdapter;
use dap::events::{ExitedEventBody, StoppedEventBody, ThreadEventBody};
use dap::responses::{
    ContinueResponse, SetBreakpointsResponse, SetExceptionBreakpointsResponse,
    StackTraceResponse, ThreadsResponse,
};
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
        eprintln!("Accepted client on: {}", addr); // changed to eprintln!
        let read_stream = BufReader::new(stream.try_clone()?);
        let write_stream = BufWriter::new(stream);
        let mut server = Server::new(read_stream, write_stream);

        // Get the adapter from the init function
        let adapter = multi_session_init(&mut server)?;

        // Run the server using the adapter
        run_server(&mut server, adapter)?;
    } else {
        let path = opts.file.ok_or(MyAdapterError::MissingFile)?;
        let file = File::open(path)?;
        let adapter = MyAdapter::new(file);
        eprintln!("running single-session");
        let write = BufWriter::new(stdout());
        let read = BufReader::new(stdin());
        let mut server = Server::new(read, write);
        run_server(&mut server, adapter)?;
    }
    eprintln!("exited run_Server");
    Ok(())
}

fn multi_session_init<R, W>(
    server: &mut Server<R, W>,
) -> AdapterResult<MyAdapter>
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

        unknown_command => {
            return Err(MyAdapterError::UnhandledCommandError(
                unknown_command.clone(),
            ));
        }
    }

    // handle the second request (Launch)
    let req = match server.poll_request()? {
        Some(req) => req,
        None => return Err(MyAdapterError::MissingCommandError),
    };

    let program_path = if let Command::Launch(params) = &req.command {
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
        panic!("second request was not a launch");
    };

    // Open file using the extracted program path
    let file = File::open(program_path)?;

    // Construct the adapter
    let mut adapter = MyAdapter::new(file);

    //NOT SURE IF WE NEED TWO THREADS
    let thread = &adapter.create_thread(String::from("Main"));
    let thread2 = &adapter.create_thread(String::from("Thread 1"));

    //FIND OUT IF WE NEED TO SEND EVENTS AS STILL WORKS WITHOUT IT
    server.send_event(Event::Thread(ThreadEventBody {
        reason: types::ThreadEventReason::Started,
        thread_id: thread.id,
    }))?;

    //Notify server of second thread
    server.send_event(Event::Thread(ThreadEventBody {
        reason: types::ThreadEventReason::Started,
        thread_id: thread2.id,
    }))?;

    // Return the adapter instead of running the server
    Ok(adapter)
}

fn run_server<R: Read, W: Write>(
    server: &mut Server<R, W>,
    mut adapter: MyAdapter,
) -> AdapterResult<()> {
    loop {
        // Start looping here
        let req = match server.poll_request()? {
            Some(req) => req,
            None => return Err(MyAdapterError::MissingCommandError),
        };
        match dbg!(&req.command) {
            Command::Launch(_) => {
                let rsp = req.success(ResponseBody::Launch);
                server.respond(rsp)?;
            }

            Command::SetBreakpoints(args) => {
                //Add breakpoints
                if let Some(breakpoint) = &args.breakpoints {
                    let out =
                        adapter.set_breakpoint(args.source.clone(), breakpoint);

                    //Success
                    let rsp = req.success(ResponseBody::SetBreakpoints(
                        SetBreakpointsResponse { breakpoints: (out) },
                    ));
                    server.respond(rsp)?;
                }
            }

            //TODO: Implement this request fully when adapter becomes functional
            Command::SetExceptionBreakpoints(_) => {
                let rsp = req.success(ResponseBody::SetExceptionBreakpoints(
                    SetExceptionBreakpointsResponse {
                        breakpoints: (None),
                    },
                ));
                server.respond(rsp)?;
            }

            //Retrieve a list of all threads
            Command::Threads => {
                let rsp = req.success(ResponseBody::Threads(ThreadsResponse {
                    threads: adapter.clone_threads(),
                }));
                server.respond(rsp)?;
            }

            // Disconnect the server AND exit the debugger
            Command::Disconnect(_) => {
                let rsp = req.success(ResponseBody::Disconnect);
                server.send_event(Event::Exited(ExitedEventBody {
                    exit_code: 0,
                }))?;
                server.respond(rsp)?;
                return Err(MyAdapterError::ExitError(0));
            }

            //TODO: SEE IF PROPER IMPLEMENTATION OF STACK_FRAMES AND TOTAL_FRAMES IS NEEDED
            Command::StackTrace(_args) => {
                let rsp =
                    req.success(ResponseBody::StackTrace(StackTraceResponse {
                        stack_frames: vec![],
                        total_frames: None,
                    }));
                server.respond(rsp)?;
            }
            // TODO: ASK ABOUT STEPPING GRANUALARITY
            //Continue the debugger
            Command::Continue(_args) => {
                let rsp =
                    req.success(ResponseBody::Continue(ContinueResponse {
                        all_threads_continued: None,
                    }));
                server.respond(rsp)?;
            }

            //Send a Stopped event with reason Pause
            Command::Pause(args) => {
                //Get ID before rsp takes ownership
                let thread_id = args.thread_id;
                let rsp = req.success(ResponseBody::Pause);
                //Send response first
                server.respond(rsp)?;
                //Send event
                server.send_event(Event::Stopped(StoppedEventBody {
                    reason: types::StoppedEventReason::Pause,
                    description: Some(String::from("Paused")),
                    preserve_focus_hint: None,
                    text: None,
                    thread_id: Some(thread_id),
                    all_threads_stopped: None,
                    hit_breakpoint_ids: None,
                }))?;
            }

            // Step over
            Command::Next(args) => {
                //Get ID before rsp takes ownership
                let thread_id = args.thread_id;
                let rsp = req.success(ResponseBody::Next);
                // Send response first
                server.respond(rsp)?;
                // Send event
                server.send_event(Event::Stopped(StoppedEventBody {
                    reason: types::StoppedEventReason::Step,
                    description: Some(String::from("Continue")),
                    thread_id: Some(thread_id),
                    preserve_focus_hint: None,
                    text: None,
                    all_threads_stopped: None,
                    hit_breakpoint_ids: None,
                }))?;
            }
            // Step in
            //TODO: SEE IF WE NEED TO IMPLEMENT WHEN STEPIN BEHAVES LIKE NEXT
            Command::StepIn(args) => {
                //Get ID before rsp takes ownership
                let thread_id = args.thread_id;
                // Send response first
                let rsp = req.success(ResponseBody::StepIn);
                server.respond(rsp)?;
                // Send event
                server.send_event(Event::Stopped(StoppedEventBody {
                    reason: types::StoppedEventReason::Step,
                    description: Some(String::from("Paused on step")),
                    thread_id: Some(thread_id),
                    preserve_focus_hint: None,
                    text: None,
                    all_threads_stopped: None,
                    hit_breakpoint_ids: None,
                }))?;
            }

            Command::StepOut(args) => {
                //Get ID before rsp takes ownership
                let thread_id = args.thread_id;
                // Send response first
                let rsp = req.success(ResponseBody::StepOut);
                server.respond(rsp)?;
                // Send event
                server.send_event(Event::Stopped(StoppedEventBody {
                    reason: types::StoppedEventReason::Step,
                    description: Some(String::from("Paused on step")),
                    thread_id: Some(thread_id),
                    preserve_focus_hint: None,
                    text: None,
                    all_threads_stopped: None,
                    hit_breakpoint_ids: None,
                }))?;
            }
            unknown_command => {
                return Err(MyAdapterError::UnhandledCommandError(
                    unknown_command.clone(),
                ));
            }
        }
    }
}
