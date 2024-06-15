use calyx_utils::{CalyxResult, Error};

pub fn apply_preprocessor(text: &str) -> CalyxResult<String> {
    let mut ctx = preprocessor::Context::new();
    let result = text
        .lines()
        .map(|line| {
            ctx.process(line.into())
                .map_err(|err| Error::misc(err.to_string()))
        })
        .collect::<CalyxResult<Vec<_>>>()?;

    Ok(result.join("\n"))
}
