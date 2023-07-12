use std::fs::File;

pub struct MyAdapter;

//will change that later, will add the file to the field of the struct
impl MyAdapter {
    pub fn new(file: File) -> Self {
        MyAdapter
    }
}
