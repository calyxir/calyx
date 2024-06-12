use std::process::Command;

/// Returns the standard output from executing a given command `cmd` with
/// arguments `args`. Fails when the command fails or succeeds but with a
/// nonzero exit code (and `wants_zero`).
pub fn capture_command_stdout(
    cmd: &str,
    args: &[&str],
    wants_zero: bool,
) -> std::io::Result<String> {
    let output = Command::new(cmd).args(args).output()?;
    if !output.status.success() && wants_zero {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "Command `{} {}` did not execute successfully (code={}). Stderr: {}",
                cmd,
                args.iter()
                    .map(|str| str.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
                output.status.code().unwrap(),
                String::from_utf8(output.stderr).unwrap_or_default()
            ),
        ))
    } else {
        String::from_utf8(output.stdout)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}
