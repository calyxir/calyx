use num_ir::typing::Endian;
use num_ir::{short::TryFromShort, typing::TypeSpec};
use rustyline::{DefaultEditor, Result};

fn handle_info(s: &str) {
    let Ok(e) = TypeSpec::read_short_t(s) else {
        println!("can't parse {} into type", s);
        return;
    };
    println!("{:?}", e)
}

fn main() -> Result<()> {
    let mut rl = DefaultEditor::new()?;
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                let t: Vec<_> = line.split_ascii_whitespace().collect();
                if t[0] == "help" {
                    println!("info <type>: check if a type is parseable");
                    println!(
                        "<type> <number>: prints a number as hex and pretty-printed. <number> can be a hexstring starting with 0x"
                    );
                    continue;
                }
                if t.len() < 2 {
                    println!("expecting at least two space-separated elements");
                    continue;
                }
                if t[0] == "info" {
                    handle_info(t[1]);
                    continue;
                }
                let Ok(e) = TypeSpec::read_short_t(t[0]) else {
                    println!("can't parse {} into type", t[0]);
                    continue;
                };
                let Ok(res) = e.read_str(t[1], Endian::Little) else {
                    println!("can't read {} into type {}", t[1], t[0]);
                    continue;
                };
                println!("\t= {}", e.write_hexstring(&res, Endian::Little));
                println!("\t= {}", e.write_string(&res, Endian::Little))
            }
            Err(_) => {
                break;
            }
        }
    }

    Ok(())
}
