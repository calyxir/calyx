use cargo_lock::Lockfile;
use std::fs::File;
use std::io::Write;

fn main() {
    let lockfile = Lockfile::load("Cargo.lock").unwrap();
    let package: cargo_lock::Package = lockfile
        .packages
        .into_iter()
        .find(|pkg| pkg.name.as_str() == "calyx")
        .unwrap();
    let git_hash = format!("{}", package.source.unwrap().precise().unwrap());
    let json = format!("{{ \"version\": \"{}\" }}", git_hash);

    let mut file = File::create("calyx_hash.json").unwrap();
    file.write_all(json.as_bytes()).unwrap();
}
