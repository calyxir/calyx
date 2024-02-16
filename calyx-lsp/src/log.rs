#[cfg(feature = "log")]
use chrono::Local;
#[cfg(feature = "log")]
use std::fs::OpenOptions;
#[cfg(feature = "log")]
use std::io::Write;

pub struct Debug;

impl Debug {
    #[allow(unused)]
    pub fn stdout<S: AsRef<str>>(msg: S) {
        #[cfg(feature = "log")]
        {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(format!("/tmp/calyx-lsp-debug.log"))
                .unwrap();
            writeln!(file, "{}", msg.as_ref()).expect("Unable to write file");
        }
    }

    #[allow(unused)]
    pub fn init<S: AsRef<str>>(msg: S) {
        #[cfg(feature = "log")]
        {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(format!("/tmp/calyx-lsp-debug.log"))
                .unwrap();
            writeln!(file, "{} {}", msg.as_ref(), Local::now().to_rfc2822())
                .expect("Unable to write file");
        }
    }

    #[allow(unused)]
    pub fn update<S: AsRef<str>>(name: &str, msg: S) {
        #[cfg(feature = "log")]
        {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(format!("/tmp/calyx-lsp-debug-{name}.log"))
                .unwrap();
            writeln!(file, "{}", msg.as_ref()).expect("Unable to write file");
        }
    }
}

macro_rules! stdout {
    ($($t:tt)*) => {{
        log::Debug::stdout(format!($($t)*))
    }};
}

pub(crate) use stdout;
