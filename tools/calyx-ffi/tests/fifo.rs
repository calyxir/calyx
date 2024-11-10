use calyx_ffi::cider_ffi_backend;
use calyx_ffi::prelude::*;

enum QueueCommand {
    Pop = 0,
    Push = 1,
}

#[derive(PartialEq, Eq, Debug)]
enum QueueStatus {
    Ok = 0,
    Err = 1,
}

calyx_ffi::declare_interface! {
    Queue(cmd: 1, value: 32) -> (ans: 32, err: 1) impl {
        fn status(&mut self) -> QueueStatus {
            if self.err() == 0 { QueueStatus::Ok } else { QueueStatus::Err }
        }

        fn assert_no_error(&mut self) {
            assert_eq!(QueueStatus::Ok, self.status(), "queue underflowed or overflowed");
        }

        fn push(&mut self, value: u32) {
            self.reset();
            self.set_cmd(QueueCommand::Push as u64);
            self.set_value(value as u64);
            self.go();
            self.assert_no_error();
        }

        fn pop(&mut self) -> u32 {
            self.reset();
            self.set_cmd(QueueCommand::Pop as u64);
            self.go();
            self.assert_no_error();
            self.ans() as u32
        }
    }
}

#[calyx_ffi(
    src = "tests/fifo.futil",
    comp = "main",
    backend = cider_ffi_backend,
    derive = [
        Queue(cmd: 1, value: 32) -> (ans: 32, err: 1)
    ]
)]
struct Fifo;

#[cfg(test)]
#[calyx_ffi_tests]
mod tests {
    use super::*;

    #[calyx_ffi_test]
    fn test_fifo(fifo: &mut Fifo) {
        println!("testing fifo");

        fifo.push(1);
        fifo.push(2);
        assert_eq!(1, fifo.pop());
        fifo.push(3);
        assert_eq!(2, fifo.pop());
        assert_eq!(3, fifo.pop());
    }
}
