use std::fs::File;
pub struct MyAdapter {
    file: File,
    // Other fields of the struct
}

impl MyAdapter {
    pub fn new(file: File) -> Self {
        MyAdapter { file }
    }
}
