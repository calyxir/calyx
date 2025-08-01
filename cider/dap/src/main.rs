mod adapter;
mod error;

use adapter::MyAdapter;
use dap::events::{ExitedEventBody, StoppedEventBody, ThreadEventBody};
use dap::responses::{
    ContinueResponse, ScopesResponse, SetBreakpointsResponse,
    SetExceptionBreakpointsResponse, StackTraceResponse, ThreadsResponse,
};
use error::MyAdapterError;

use dap::prelude::*;
use error::AdapterResult;
use responses::VariablesResponse;
use slog::{Drain, info};
//use std::default;
use std::fs::OpenOptions;
use std::io::{BufReader, BufWriter, Read, Write, stdin, stdout};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

#[derive(argh::FromArgs)]
/// Positional arguments for file path
struct Opts {
    #[argh(switch, long = "tcp")]
    /// runs in tcp mode
    is_multi_session: bool,
    #[argh(option, short = 'p', long = "port", default = "8080")]
    /// port for the TCP server
    port: u16,
    #[argh(
        option,
        short = 'l',
        default = "Path::new(option_env!(\"CALYX_PRIMITIVES_DIR\").unwrap_or(\".\")).into()"
    )]
    /// std_lib path
    path: PathBuf,
}

fn main() -> Result<(), MyAdapterError> {
    let opts: Opts = argh::from_env();
    // Initializing logger
    let log_path = "/tmp/output.log"; // Stores in tmp file for now, if testing, use a relative path
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(log_path)
        .unwrap();

    // Different decorators and drains for terminal and file logging -- async_drain picks the right one based on session
    let async_drain = if opts.is_multi_session {
        let term_decorator = slog_term::TermDecorator::new().build();
        let term_drain =
            slog_term::FullFormat::new(term_decorator).build().fuse();
        slog_async::Async::new(term_drain).build().fuse()
    } else {
        let file_decorator = slog_term::PlainDecorator::new(file);
        let file_drain =
            slog_term::FullFormat::new(file_decorator).build().fuse();
        slog_async::Async::new(file_drain).build().fuse()
    };
    let logger = slog::Logger::root(async_drain, slog::o!());

    info!(logger, "Logging initialized");
    if opts.is_multi_session {
        info!(logger, "running multi-session");
        let listener = TcpListener::bind(("127.0.0.1", opts.port))?;
        info!(logger, "bound on port: {} ", opts.port);
        let (stream, addr) = listener.accept()?;
        info!(logger, "Accepted client on: {}", addr);
        let read_stream = BufReader::new(stream.try_clone()?);
        let write_stream = BufWriter::new(stream);
        let mut server = Server::new(read_stream, write_stream);
        // Get the adapter from the init function
        let adapter = multi_session_init(&mut server, &logger, opts.path)?;
        run_server(&mut server, adapter, &logger)?;
    } else {
        info!(logger, "running single-session");
        let write = BufWriter::new(stdout());
        let read = BufReader::new(stdin());
        let mut server = Server::new(read, write);
        let adapter = multi_session_init(&mut server, &logger, opts.path)?; //i dont think this is right
        run_server(&mut server, adapter, &logger)?;
    }
    info!(logger, "exited run_Server");
    Ok(())
}

fn multi_session_init<R, W>(
    server: &mut Server<R, W>,
    logger: &slog::Logger,
    std_path: PathBuf,
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
                    // Not sure if we need it
                    // Make VSCode send disassemble request
                    supports_stepping_granularity: Some(true),
                    ..Default::default()
                }));
            server.respond(rsp)?;
            server.send_event(Event::Initialized)?;
        }

        unknown_command => {
            return Err(MyAdapterError::UnhandledCommandError(Box::new(
                unknown_command.clone(),
            )));
        }
    }

    // handle the second request (Launch)
    //seems like second request doesn't necessarily need to be a launch request
    let req = match server.poll_request()? {
        Some(req) => req,
        None => return Err(MyAdapterError::MissingCommandError),
    };

    let program_path = if let Command::Launch(params) = &req.command {
        if let Some(data) = &params.additional_data {
            if let Some(program_path) = data.get("program") {
                info!(logger, "Program path: {}", program_path);
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

    // Construct the adapter
    let mut adapter = MyAdapter::new(program_path, std_path)?;

    // one thread idk why but it works
    let thread = &adapter.create_thread(String::from("Main")); //does not seem as though this does anything

    // Notify server of first thread
    server.send_event(Event::Thread(ThreadEventBody {
        reason: types::ThreadEventReason::Started,
        thread_id: thread.id,
    }))?;

    // Return the adapter instead of running the server
    Ok(adapter)
}

fn run_server<R: Read, W: Write>(
    server: &mut Server<R, W>,
    mut adapter: MyAdapter,
    logger: &slog::Logger,
) -> AdapterResult<()> {
    let stopped = create_stopped(
        types::StoppedEventReason::Entry,
        String::from("Debugger has started"),
        0,
        true,
    );
    server.send_event(stopped)?;
    info!(logger, "sent stopped event (initialization)");

    loop {
        // Start looping here
        let req = match server.poll_request()? {
            Some(req) => req,
            None => return Err(MyAdapterError::MissingCommandError),
        };
        info!(logger, "{req:?}");
        match &req.command {
            Command::Launch(_) => {
                //why is this listed a second time
                let rsp = req.success(ResponseBody::Launch);
                server.respond(rsp)?;
            }

            Command::SetBreakpoints(args) => {
                // Add breakpoints
                if let Some(bkpts) = &args.breakpoints {
                    let out =
                        adapter.handle_breakpoint(args.source.clone(), bkpts);

                    // Success
                    let rsp = req.success(ResponseBody::SetBreakpoints(
                        SetBreakpointsResponse { breakpoints: (out) },
                    ));
                    server.respond(rsp)?;
                }
            }

            Command::SetExceptionBreakpoints(_) => {
                let rsp = req.success(ResponseBody::SetExceptionBreakpoints(
                    SetExceptionBreakpointsResponse {
                        breakpoints: (None),
                    },
                ));
                server.respond(rsp)?;
            }
            // Retrieve a list of all threads
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

                //Exit
                info!(logger, "exited debugger");
                return Ok(());
            }
            // Send StackTrace, may be useful to make it more robust in the future
            Command::StackTrace(_args) => {
                let frames = adapter.get_stack();
                let rsp =
                    req.success(ResponseBody::StackTrace(StackTraceResponse {
                        stack_frames: frames,
                        total_frames: Some(0),
                    }));
                server.respond(rsp)?;
            }
            // Continue the debugger
            Command::Continue(args) => {
                // need to run debugger, ngl not really sure how to implement this functionality
                // run debugger until breakpoint or paused -> maybe have a process to deal w running debugger?
                let stopped = adapter.on_continue(args.thread_id);
                let rsp =
                    req.success(ResponseBody::Continue(ContinueResponse {
                        all_threads_continued: Some(true),
                    }));
                server.respond(rsp)?;
                server.send_event(stopped)?;
            }
            // Send a Stopped event with reason Pause
            Command::Pause(args) => {
                //necessary to clear out object references
                adapter.on_pause();

                // Get ID before rsp takes ownership
                // need to communicate pause to debugger
                let thread_id = args.thread_id;
                let rsp = req.success(ResponseBody::Pause);
                // Send response first
                server.respond(rsp)?;
                // Send event
                let stopped = create_stopped(
                    types::StoppedEventReason::Pause,
                    String::from("Paused"),
                    thread_id,
                    false,
                );
                server.send_event(stopped)?;
            }
            // Step over
            Command::Next(args) => {
                // Move stack frame
                // If done then disconnect
                if adapter.next_line(args.thread_id) {
                    let rsp = req.clone().success(ResponseBody::Disconnect);
                    server.send_event(Event::Exited(ExitedEventBody {
                        exit_code: 0,
                    }))?;
                    server.respond(rsp)?;

                    // Exit
                    info!(logger, "exited debugger");
                    return Ok(());
                }

                // Get ID before rsp takes ownership
                let thread_id = args.thread_id;
                let rsp = req.success(ResponseBody::Next);
                // Send response first
                server.respond(rsp)?;
                // Send event
                let stopped = create_stopped(
                    types::StoppedEventReason::Step,
                    String::from("Continue"),
                    thread_id,
                    false,
                );
                server.send_event(stopped)?;
            }
            // Step in
            Command::StepIn(args) => {
                // Get ID before rsp takes ownership
                let thread_id = args.thread_id;
                // Send response first
                let rsp = req.success(ResponseBody::StepIn);
                server.respond(rsp)?;
                // Send event
                let stopped = create_stopped(
                    types::StoppedEventReason::Step,
                    String::from("Paused on step"),
                    thread_id,
                    false,
                );
                server.send_event(stopped)?;
            }
            // Step out
            Command::StepOut(args) => {
                // Get ID before rsp takes ownership
                let thread_id = args.thread_id;
                // Send response first
                let rsp = req.success(ResponseBody::StepOut);
                server.respond(rsp)?;
                // Send event
                let stopped = create_stopped(
                    types::StoppedEventReason::Step,
                    String::from("Paused on step"),
                    thread_id,
                    false,
                );
                server.send_event(stopped)?;
            }
            Command::Scopes(args) => {
                let frame_id = args.frame_id;
                let rsp = req.success(ResponseBody::Scopes(ScopesResponse {
                    scopes: adapter.get_scopes(frame_id),
                }));
                server.respond(rsp)?;
            }
            Command::Variables(args) => {
                let var_ref = args.variables_reference;
                let rsp =
                    req.success(ResponseBody::Variables(VariablesResponse {
                        variables: adapter.get_variables(var_ref),
                    }));
                server.respond(rsp)?;
            }

            unknown_command => {
                return Err(MyAdapterError::UnhandledCommandError(Box::new(
                    unknown_command.clone(),
                )));
            }
        }
    }
}

///Helper function used to create a Stopped event
fn create_stopped(
    reason: types::StoppedEventReason,
    desc: String,
    thread_id: i64,
    all_threads: bool,
) -> Event {
    let thread = match all_threads {
        true => None,
        false => Some(thread_id),
    };
    Event::Stopped(StoppedEventBody {
        reason,
        description: Some(desc),
        thread_id: thread,
        preserve_focus_hint: None,
        text: None,
        all_threads_stopped: Some(all_threads),
        hit_breakpoint_ids: None,
    })
}
