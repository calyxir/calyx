use std::fs::File;
use std::io::{BufReader, Read};

use cider_serde as cs;

fn main() {
    let cider_header = {
        let mut header_file =
            File::open("cider-header").expect("can't open cider header");
        let mut raw_header = vec![];
        header_file
            .read_to_end(&mut raw_header)
            .expect("can't read to end");

        cs::DataHeader::deserialize(&raw_header).expect("can't deserde")
    };

    let text_header = {
        let header_file =
            File::open("text-header").expect("can't open text header");
        let header_r = BufReader::new(header_file);
        conv_formats::dat_dir::read_header(header_r).unwrap()
    };

    // println!("{:?}", cider_header);

    // assert whether number of mems is the same
    assert_eq!(text_header.len(), cider_header.memories.len());
}
