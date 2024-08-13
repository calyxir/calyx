use std::fs::File;
use std::io::{self, Write};

pub struct BinaryOutputFile {
    file: File,
    bit_buffer: u8,
    bit_count: u8,
}

impl BinaryOutputFile {
    // Constructor to create and open the binary file
    fn new(file_path: &str) -> io::Result<Self> {
        let file = File::create(file_path)?;
        Ok(Self {
            file,
            bit_buffer: 0,
            bit_count: 0,
        })
    }

    // Method to write bits to the binary file
    fn write_bits(&mut self, bits: &[u8]) -> io::Result<()> {
        for &bit in bits {
            // Shift the buffer left by 1 and add the new bit
            self.bit_buffer = (self.bit_buffer << 1) | bit;
            self.bit_count += 1;

            if self.bit_count == 8 {
                // Buffer is full (8 bits), write it to the file
                self.file.write_all(&[self.bit_buffer])?;
                self.bit_buffer = 0; // Reset the buffer
                self.bit_count = 0;  // Reset the bit count
            }
        }
        Ok(())
    }

    // Method to flush any remaining bits in the buffer to the file
    fn flush(&mut self) -> io::Result<()> {
        if self.bit_count > 0 {
            // There are leftover bits, pad the buffer with zeros and write it
            self.bit_buffer <<= 8 - self.bit_count; // Shift left to pad any leftover bits with zeros
            self.file.write_all(&[self.bit_buffer])?;
            self.bit_buffer = 0; 
            self.bit_count = 0;  
        }
        Ok(())
    }
}

