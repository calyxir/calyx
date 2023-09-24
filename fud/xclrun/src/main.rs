mod xrt;

fn main() {
    println!("Hello, world!");
    unsafe {
        xrt::xrtDeviceOpen(0);
    }
}
