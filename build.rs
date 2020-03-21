use clap::Shell;
include!("src/cmdline.rs");

fn main() {
    let mut app = Opts::clap();
    match option_env!("CALYX_AC_ZSH") {
        None => (),
        Some(dir) => {
            app.gen_completions(env!("CARGO_PKG_NAME"), Shell::Zsh, dir)
        }
    }
    match option_env!("CALYX_AUTOCOMPLETION_BASH") {
        None => (),
        Some(dir) => {
            app.gen_completions(env!("CARGO_PKG_NAME"), Shell::Bash, dir)
        }
    }
}
