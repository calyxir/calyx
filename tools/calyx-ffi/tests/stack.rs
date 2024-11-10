use calyx_ffi::cider_ffi_backend;
use calyx_ffi::prelude::*;

enum StackCommand {
    Push = 0,
    Pop = 1,
}

calyx_ffi::declare_interface! {
    Stack(cmd: 1, value: 32) -> (out: 32) impl {
        fn push(&mut self, value: u32) {
            self.reset();
            self.set_cmd(StackCommand::Push as u64);
            self.set_value(value as u64);
            self.go();
        }

        fn pop(&mut self) -> u32 {
            self.reset();
            self.set_cmd(StackCommand::Pop as u64);
            self.go();
            self.out() as u32
        }
    }
}

#[calyx_ffi(
    src = "tests/stack.futil",
    comp = "main",
    backend = cider_ffi_backend,
    derive = [
        Stack(cmd: 1, value: 32) -> (out: 32)
    ]
)]
struct ReallyBadStack;

#[cfg(test)]
#[calyx_ffi_tests]
mod tests {
    use super::*;

    #[calyx_ffi_test]
    fn test_stack(stack: &mut ReallyBadStack) {
        println!("testing fifo");

        stack.push(1);
        stack.push(2);
        assert_eq!(2, stack.pop());
        // fifo.push(3);
        // assert_eq!(3, fifo.pop());
        assert_eq!(1, stack.pop());
    }
}
