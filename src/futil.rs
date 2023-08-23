use calyx::driver;
use calyx_utils::CalyxResult;

fn main() -> CalyxResult<()> {
    driver::run_compiler()?;
    log::warn!("The `futil` binary is deprecated. Please use `calyx` instead.");
    Ok(())
}
