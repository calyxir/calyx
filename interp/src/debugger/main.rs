struct MyStruct {
    prop: usize,
}

struct Point(f32, f32);

fn main() {
    let a = 42;
    let b = vec![0, 0, 0, 100];
    let c = [1, 2, 3, 4, 5];
    let d = 0x5ff;
    let e = MyStruct { prop: 10 };
    let p = Point(3.14, 3.14);

    println!("Hello, world!");
}
