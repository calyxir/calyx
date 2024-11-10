use calyx_ffi::cider_ffi_backend;
use calyx_ffi::prelude::*;

enum StackCommand {
    Push = 0,
    Pop = 1,
}

const STACK_CAPACITY: u64 = 16;

calyx_ffi::declare_interface! {
    Stack(cmd: 1, value: 32) -> (out: 32, length: 4) impl {
        fn push(&mut self, value: u32) {
            assert!(self.length() < STACK_CAPACITY, "tried to push when length={}", STACK_CAPACITY);
            println!("stack has length {} before push", self.length());
            let old_length = self.length();
            self.set_cmd(StackCommand::Push as u64);
            self.set_value(value as u64);
            self.go();
            assert_eq!(old_length + 1, self.length(), "stack length should increase by 1 on push");
        }

        fn pop(&mut self) -> u32 {
            assert!(self.length() > 0, "tried to pop when stack empty");
            println!("stack has length {} before pop", self.length());
            let old_length = self.length();
            self.set_cmd(StackCommand::Pop as u64);
            self.go();
            assert_eq!(old_length - 1, self.length(), "stack length should decrease by 1 on pop");
            self.out() as u32
        }
    }
}

#[calyx_ffi(
    src = "tests/stack.futil",
    comp = "main",
    backend = cider_ffi_backend,
    derive = [
        Stack(cmd: 1, value: 32) -> (out: 32, length: 4)
    ]
)]
struct ReallyBadStack;

#[cfg(test)]
#[calyx_ffi_tests]
mod tests {
    use super::*;

    #[calyx_ffi_test]
    fn test_stack(stack: &mut ReallyBadStack) {
        println!("testing stack");

        stack.push(1);
        stack.push(2);
        assert_eq!(2, stack.pop());
        stack.push(3);
        assert_eq!(3, stack.pop());
        assert_eq!(1, stack.pop());
    }
}
