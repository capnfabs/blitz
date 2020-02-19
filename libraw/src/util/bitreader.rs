use std::cmp::min;
use std::io;

pub struct BitReader<T: io::Read> {
    reader: T,
    buffer: u32,
    offset: u32,
    total_read: usize,
}

#[inline]
fn eof<T>() -> io::Result<T> {
    Err(io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "Underlying buffer is empty",
    ))
}

impl<T: io::Read> BitReader<T> {
    pub fn new(reader: T) -> Self {
        BitReader {
            reader,
            buffer: 0,
            offset: 32,
            total_read: 0,
        }
    }

    /// Returns true if the buffer has content in it after the method call, false if it's EOF.
    #[inline]
    fn ensure_buffer_filled(&mut self) -> io::Result<bool> {
        if self.offset != 32 {
            return Ok(true);
        }
        let mut buf = [0u8; 4];
        let count = self.reader.read(&mut buf)?;

        if count == 0 {
            return Ok(false);
        }

        self.total_read += count;

        // If we didn't manage to fill the buffer completely, then right-align bytes
        // in the buffer so that we're not making up bits at the end of the stream.
        let bits_read = count * 8;
        let new_offset = (32 - bits_read) as u32;
        let num = u32::from_be_bytes(buf);
        self.buffer = num >> new_offset;
        self.offset = new_offset;
        Ok(true)
    }

    pub fn total_read(&self) -> usize {
        self.total_read
    }

    pub fn count_continuous_0s(&mut self) -> io::Result<u32> {
        let mut counted_0s_total = 0;
        loop {
            if !self.ensure_buffer_filled()? {
                return eof();
            }
            let counted_this_loop =
                (self.buffer << self.offset | ((1 << self.offset) - 1)).leading_zeros();
            self.offset += counted_this_loop;
            counted_0s_total += counted_this_loop;
            // Didn't read to end of buffer, which means we have enough info
            // and we don't need to check the next byte.
            if self.offset < 32 {
                return Ok(counted_0s_total);
            }
        }
    }

    pub fn count_continuous_1s(&mut self) -> io::Result<u32> {
        let mut counted_1s_total = 0;
        loop {
            if !self.ensure_buffer_filled()? {
                return eof();
            }
            let counted_this_loop = (!(self.buffer << self.offset)).leading_zeros();
            self.offset += counted_this_loop;
            counted_1s_total += counted_this_loop;
            // Didn't read to end of buffer, which means we have enough info
            // and we don't need to check the next byte.
            if self.offset < 32 {
                return Ok(counted_1s_total);
            }
        }
    }

    pub fn read_bits(&mut self, count: usize) -> io::Result<u32> {
        // not sure how it works at the margin yet
        assert!(count < 30);
        let count = count as u32;
        if count == 0 {
            return Ok(0);
        }
        let mut value = 0;
        let mut bits_remaining = count;
        loop {
            if !self.ensure_buffer_filled()? {
                return eof();
            }
            let bits_available = 32 - self.offset;
            let bits_to_read_this_buffer = min(bits_available, bits_remaining);
            let bit_mask = (1 << bits_to_read_this_buffer) - 1;
            let right_shift = bits_available.saturating_sub(bits_to_read_this_buffer);
            value <<= bits_to_read_this_buffer;
            let from_this_buffer = (self.buffer >> right_shift) & bit_mask;
            value |= from_this_buffer;
            bits_remaining -= bits_to_read_this_buffer;
            self.offset += bits_to_read_this_buffer;
            if bits_remaining == 0 {
                break;
            }
        }
        Ok(value)
    }
}

#[cfg(test)]
mod test {
    use crate::util::bitreader::BitReader;
    use hex;
    use std::io::Result;

    #[test]
    fn read_bits() {
        let data: Vec<u8> = hex::decode("7775626261206C756262612064756220647562").unwrap();
        let mut reader = BitReader::new(data.as_slice());
        let mut rb = |count| reader.read_bits(count).unwrap();
        assert_eq!(rb(4), 0x7);
        assert_eq!(rb(4), 0x7);
        assert_eq!(rb(4), 0x7);
        assert_eq!(rb(4), 0x5);
        assert_eq!(rb(4), 0x6);
        assert_eq!(rb(4), 0x2);
        assert_eq!(rb(4), 0x6);
        // Should be at pos 28 by now, i.e. close to a boundary
        assert_eq!(rb(8), 0x26);
        assert_eq!(rb(8), 0x12);
        assert_eq!(rb(8), 0x06);
        assert_eq!(rb(8), 0xC7);
        assert_eq!(rb(8), 0x56);
    }

    #[test]
    fn read_bits_test_end() {
        let data: Vec<u8> = hex::decode("777562").unwrap();
        let mut reader = BitReader::new(data.as_slice());
        let mut rb = |count| reader.read_bits(count).unwrap();
        assert_eq!(rb(12), 0x777);
        assert_eq!(rb(12), 0x562);
    }

    #[test]
    fn count_continuous_1s() -> Result<()> {
        let data: Vec<u8> = hex::decode("FFF0100FF800").unwrap();
        let mut reader = BitReader::new(data.as_slice());
        assert_eq!(reader.count_continuous_1s()?, 12);
        for _ in 0..100 {
            // Shouldn't increment the counter, so should be able to do this
            // infinitely
            assert_eq!(reader.count_continuous_1s()?, 0);
        }
        assert_eq!(reader.read_bits(7)?, 0x00);
        assert_eq!(reader.count_continuous_1s()?, 1);
        assert_eq!(reader.read_bits(8)?, 0x00);
        // This crosses the 32-bit boundary, should be 4 before + 5 after
        assert_eq!(reader.count_continuous_1s()?, 9);
        // The last 11 bytes should be zeros
        assert_eq!(reader.read_bits(11).unwrap(), 0);
        Ok(())
    }

    #[test]
    fn count_continuous_0s_and_1s() -> Result<()> {
        let data: Vec<u8> = hex::decode("03FFF0E00FF800").unwrap();
        let mut reader = BitReader::new(data.as_slice());
        assert_eq!(reader.count_continuous_0s()?, 6);
        assert_eq!(reader.count_continuous_1s()?, 14);
        assert_eq!(reader.count_continuous_0s()?, 4);
        assert_eq!(reader.count_continuous_1s()?, 3);
        // This one crosses boundaries
        assert_eq!(reader.count_continuous_0s()?, 9);
        assert_eq!(reader.read_bits(9)?, 0x1FF);
        // TODO: can't handle situations where we're at the end of file yet.
        //assert_eq!(reader.count_continuous_0s(), 11);
        // That's the end!
        Ok(())
    }

    #[test]
    fn count_41_0s_across_3_bytes() -> Result<()> {
        let data: Vec<u8> = hex::decode("FFFFFFF00000000007").unwrap();
        let mut reader = BitReader::new(data.as_slice());

        assert_eq!(reader.count_continuous_1s()?, 28);
        assert_eq!(reader.count_continuous_0s()?, 41);
        assert_eq!(reader.read_bits(3)?, 0x7);
        Ok(())
    }
}
