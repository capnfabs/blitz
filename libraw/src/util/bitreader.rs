use std::cmp::min;
use std::io;
use std::mem::size_of;

pub struct BitReader<T: io::Read> {
    reader: T,
    buffer: usize,
    offset: usize,
    total_read: usize,
}

#[inline]
fn eof<T>() -> io::Result<T> {
    Err(io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "Underlying buffer is empty",
    ))
}

const BUFFER_BYTES: usize = size_of::<usize>();
const BUFFER_BITS: usize = BUFFER_BYTES * 8;

// I wanted to do this with a macro but apparently that would require a
// procedural macro.
const BIT_MASK: [usize; 32] = [
    (1 << 0) - 1,
    (1 << 1) - 1,
    (1 << 2) - 1,
    (1 << 3) - 1,
    (1 << 4) - 1,
    (1 << 5) - 1,
    (1 << 6) - 1,
    (1 << 7) - 1,
    (1 << 8) - 1,
    (1 << 9) - 1,
    (1 << 10) - 1,
    (1 << 11) - 1,
    (1 << 12) - 1,
    (1 << 13) - 1,
    (1 << 14) - 1,
    (1 << 15) - 1,
    (1 << 16) - 1,
    (1 << 17) - 1,
    (1 << 18) - 1,
    (1 << 19) - 1,
    (1 << 20) - 1,
    (1 << 21) - 1,
    (1 << 22) - 1,
    (1 << 23) - 1,
    (1 << 24) - 1,
    (1 << 25) - 1,
    (1 << 26) - 1,
    (1 << 27) - 1,
    (1 << 28) - 1,
    (1 << 29) - 1,
    (1 << 30) - 1,
    (1 << 31) - 1,
];

impl<T: io::Read> BitReader<T> {
    pub fn new(reader: T) -> Self {
        BitReader {
            reader,
            buffer: 0,
            offset: BUFFER_BITS,
            total_read: 0,
        }
    }

    /// Returns true if the buffer has content in it after the method call, false if it's EOF.
    #[inline(always)]
    fn ensure_buffer_filled(&mut self) -> io::Result<bool> {
        if self.offset != BUFFER_BITS {
            return Ok(true);
        }
        let mut buf = [0u8; BUFFER_BYTES];
        let count = self.reader.read(&mut buf)?;

        if count == 0 {
            return Ok(false);
        }

        self.total_read += count;

        // If we didn't manage to fill the buffer completely, then right-align bytes
        // in the buffer so that we're not making up bits at the end of the stream.
        let bits_read = count * 8;
        let new_offset = BUFFER_BITS - bits_read;
        let num = usize::from_be_bytes(buf);
        self.buffer = num >> new_offset;
        self.offset = new_offset;
        Ok(true)
    }

    pub fn total_read(&self) -> usize {
        self.total_read
    }

    // This *also* reads off the 1 after the zeros.
    #[inline(always)]
    pub fn count_continuous_0s(&mut self) -> io::Result<u32> {
        let mut counted_0s_total: u32 = 0;
        loop {
            if !self.ensure_buffer_filled()? {
                return eof();
            }
            let counted_this_loop =
                (self.buffer << self.offset | ((1 << self.offset) - 1)).leading_zeros();
            self.offset += counted_this_loop as usize;
            counted_0s_total += counted_this_loop;
            // Didn't read to end of buffer, which means we have enough info
            // and we don't need to check the next byte.
            if self.offset < BUFFER_BITS {
                // Skip the terminating '1'
                self.offset += 1;
                return Ok(counted_0s_total);
            }
        }
    }

    // This also reads off the 0 after the 1s!
    #[inline(always)]
    pub fn count_continuous_1s(&mut self) -> io::Result<u32> {
        let mut counted_1s_total = 0;
        loop {
            if !self.ensure_buffer_filled()? {
                return eof();
            }
            let counted_this_loop = (!(self.buffer << self.offset)).leading_zeros();
            self.offset += counted_this_loop as usize;
            counted_1s_total += counted_this_loop;
            // Didn't read to end of buffer, which means we have enough info
            // and we don't need to check the next byte.
            if self.offset < BUFFER_BITS {
                // Skip the terminating '0'
                self.offset += 1;
                return Ok(counted_1s_total);
            }
        }
    }

    #[inline(always)]
    pub fn read_bits(&mut self, count: usize) -> io::Result<u32> {
        // not sure how it works at the margin yet
        debug_assert!(count <= 30);
        if count == 0 {
            return Ok(0);
        }
        let mut value = 0;
        let mut bits_remaining = count;
        loop {
            if !self.ensure_buffer_filled()? {
                return eof();
            }
            let bits_available = BUFFER_BITS - self.offset;
            let bits_to_read_this_buffer = min(bits_available, bits_remaining);
            let bit_mask = BIT_MASK[bits_to_read_this_buffer];
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
        Ok(value as u32)
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
        let data: Vec<u8> = hex::decode("00000000777562").unwrap();
        let mut reader = BitReader::new(data.as_slice());
        let mut rb = |count| reader.read_bits(count).unwrap();
        assert_eq!(rb(30), 0x0);
        assert_eq!(rb(2), 0x0);
        assert_eq!(rb(12), 0x777);
        assert_eq!(rb(12), 0x562);
    }

    #[test]
    fn count_continuous_1s() -> Result<()> {
        let data: Vec<u8> = hex::decode("00000000FFF0100FF800").unwrap();
        let mut reader = BitReader::new(data.as_slice());
        assert_eq!(reader.read_bits(30)?, 0x0);
        assert_eq!(reader.read_bits(2)?, 0x0);
        assert_eq!(reader.count_continuous_1s()?, 12);
        assert_eq!(reader.read_bits(6)?, 0x00);
        assert_eq!(reader.count_continuous_1s()?, 1);
        assert_eq!(reader.read_bits(7)?, 0x00);
        // This crosses the 32-bit boundary, should be 4 before + 5 after
        assert_eq!(reader.count_continuous_1s()?, 9);
        // The last 11 bytes should be zeros
        assert_eq!(reader.read_bits(10).unwrap(), 0);
        Ok(())
    }

    #[test]
    fn count_continuous_0s_and_1s() -> Result<()> {
        let data: Vec<u8> = hex::decode("03FFF0E00FF800").unwrap();
        let mut reader = BitReader::new(data.as_slice());
        assert_eq!(reader.count_continuous_0s()?, 6);
        assert_eq!(reader.count_continuous_1s()?, 13);
        assert_eq!(reader.count_continuous_0s()?, 3);
        assert_eq!(reader.count_continuous_1s()?, 2);
        // This one crosses boundaries
        assert_eq!(reader.count_continuous_0s()?, 8);
        assert_eq!(reader.read_bits(8)?, 0xFF);
        // TODO: test for end of file
        Ok(())
    }

    #[test]
    fn count_41_0s_across_3_bytes() -> Result<()> {
        let data: Vec<u8> = hex::decode("FFFFFFE00000000007").unwrap();
        let mut reader = BitReader::new(data.as_slice());

        assert_eq!(reader.count_continuous_1s()?, 27);
        // There's a zero read off from the previous thing
        assert_eq!(reader.count_continuous_0s()?, 41);
        assert_eq!(reader.read_bits(2)?, 0x3);
        Ok(())
    }
}
