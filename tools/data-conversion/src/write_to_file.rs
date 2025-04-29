use std::fs::File;
use std::io::{self, Write};
use std::io::stdout;

pub struct BinaryOutputFile {
    file: File,
    bit_buffer: u8,
    bit_count: u8,
}

impl BinaryOutputFile {
    // Constructor to create and open the binary file
    pub fn new(file_path: &str) -> io::Result<Self> {
        let file = File::create(file_path)?;
        Ok(Self {
            file,
            bit_buffer: 0,
            bit_count: 0,
        })
    }

    // Method to write bits to the binary file
    pub fn write_bits(&mut self, bits: &[u8]) -> io::Result<()> {
        for &bit in bits {
            // Shift the buffer left by 1 and add the new bit
            self.bit_buffer = (self.bit_buffer << 1) | (bit & 1);
            self.bit_count += 1;

            // If the buffer is full (8 bits), write it to the file
            if self.bit_count == 8 {
                self.file.write_all(&[self.bit_buffer])?;
                self.bit_buffer = 0; // Reset the buffer
                self.bit_count = 0; // Reset the bit count
            }
        }
        Ok(())
    }

    // Method to flush remaining bits in the buffer (if any)
    pub fn flush(&mut self) -> io::Result<()> {
        if self.bit_count > 0 {
            // Shift the remaining bits to the left to form a full byte
            let remaining_byte = self.bit_buffer << (8 - self.bit_count);
            self.file.write_all(&[remaining_byte])?;
            self.bit_buffer = 0; // Reset the buffer
            self.bit_count = 0; // Reset the bit count
        }
        Ok(())
    }
}

pub fn to_text_file (
    input: &str,
    file: &mut File) -> std::io::Result<()> {

    file.write_all(input.as_bytes())?;
    file.write_all(b"\n")?;

    Ok(())
    
}

pub fn to_std_out (
    input: &str,
) -> std::io::Result<()> {
    stdout().write_all(input.as_bytes())?;
    stdout().write_all(b"\n")?;

    Ok(())
}