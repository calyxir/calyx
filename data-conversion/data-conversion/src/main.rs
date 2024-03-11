use std::fs::read_to_string;
use std::fs::File;
use std::io::Write;

fn main() {
    float_to_binary("/Users/Angelica/Desktop/calyx/data-conversion/test.txt", 
    "/Users/Angelica/Desktop/calyx/data-conversion/converted.txt");
}

//Converts [filepath_get] to binary in [filepath_send]
fn float_to_binary(filepath_get: &str, filepath_send: &str) {

    // Create a file called converted.txt
    let mut converted = File::create(filepath_send).expect("creation failed");

    // Read file line by line
    for line in read_to_string(filepath_get).unwrap().lines(){

        let float_of_string: f32;
        // Convert string to float
        match line.parse::<f32> () {
            Ok(parsed_num) => {
                float_of_string = parsed_num
            }
            Err(_) => {
                panic!("Failed to parse float from string")
            }
        }
        
        //Convert float to binary
        let binary_of_float = float_of_string.to_bits();

        // Format output nicely 
        let binary_str = format!("{:032b}", binary_of_float);
        let formatted_binary_str = format!("{} {} {}", 
                                                &binary_str[0..1], // Sign bit
                                                &binary_str[1..9], // Exponent
                                                &binary_str[9..]); // Significand

        // Write binary string to the file
        converted
            .write_all(formatted_binary_str.as_bytes())
            .expect("write failed");
        converted.write_all(b"\n").expect("write failed");
    }

    println!("Successfully converted to binary in {}", filepath_send);

}

