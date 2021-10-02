use std::fs::File;
use std::io::Write;
use std::process::Command;

// Based on: https://stackoverflow.com/a/44407625/39182
fn main() {
    let out_data = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let out_str = String::from_utf8(out_data.stdout).unwrap();
    let git_hash = out_str.trim();

    let json = format!("{{ \"version\": \"{}\" }}", git_hash);

    let mut file = File::create("calyx_hash.json").unwrap();
    file.write_all(json.as_bytes()).unwrap();
}
