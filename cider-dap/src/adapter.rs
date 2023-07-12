use std::fs::File;
pub struct MyAdapter {
    #[allow(dead_code)]
    file: File,
    // Other fields of the struct
}

impl MyAdapter {
    pub fn new(file: File) -> Self {
        MyAdapter { file }
    }
}
