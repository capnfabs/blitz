use std::cmp::min;
use std::io;

pub struct BitReader<T: io::Read> {
    reader: T,
    buffer: u32,
    offset: u32,
    total_read: usize,
}

impl<T: io::Read> BitReader<T> {
    pub fn new(reader: T) -> Self {
        let mut br = BitReader {
            reader,
            buffer: 0,
            offset: 32,
            total_read: 0,
        };
        br.ensure_buffer_filled();
        br
    }

    #[inline]
    fn ensure_buffer_filled(&mut self) {
        if self.offset != 32 {
            return;
        }
        let mut buf = [0u8; 4];
        let count = self.reader.read(&mut buf).unwrap();

        // TODO: return read error
        if count == 0 {
            panic!("NO");
        }

        self.total_read += count;

        // If we didn't manage to fill the buffer completely, then right-align bytes
        // in the buffer so that we're not making up bits at the end of the stream.
        let bits_read = count * 8;
        let new_offset = (32 - bits_read) as u32;
        let num = u32::from_be_bytes(buf);
        self.buffer = num >> new_offset;
        self.offset = new_offset;
    }

    pub fn total_read(&self) -> usize {
        self.total_read
    }

    pub fn count_continuous_0s(&mut self) -> u32 {
        self.ensure_buffer_filled();
        let counted_0_bits =
            (self.buffer << self.offset | ((1 << self.offset) - 1)).leading_zeros();
        self.offset += counted_0_bits;
        if self.offset == 32 {
            // TODO: this doesn't handle if we hit the end of the buffer. If
            // we do, it should terminate the string of 1s.
            counted_0_bits + self.count_continuous_0s()
        } else {
            counted_0_bits
        }
    }

    pub fn count_continuous_1s(&mut self) -> u32 {
        self.ensure_buffer_filled();
        let counted_1_bits = (!(self.buffer << self.offset)).leading_zeros();
        self.offset += counted_1_bits;
        if self.offset == 32 {
            // TODO: this doesn't handle if we hit the end of the buffer. If
            // we do, it should terminate the string of 1s.
            counted_1_bits + self.count_continuous_1s()
        } else {
            counted_1_bits
        }
    }

    pub fn read_bits(&mut self, count: usize) -> u32 {
        if count == 0 {
            return 0;
        }
        self.ensure_buffer_filled();
        // not sure how it works at the margin yet
        assert!(count < 30);
        let count = count as u32;

        let bits_available = 32 - self.offset;
        let bits_to_read_from_current_buffer = min(bits_available, count);
        let bits_to_read_from_next_buffer = count - bits_to_read_from_current_buffer;

        let bit_mask = (1 << bits_to_read_from_current_buffer) - 1;
        let right_shift = bits_available.saturating_sub(bits_to_read_from_current_buffer);
        let from_this_buffer = (self.buffer >> right_shift) & bit_mask;
        self.offset += bits_to_read_from_current_buffer;

        if bits_to_read_from_next_buffer == 0 {
            from_this_buffer
        } else {
            from_this_buffer << bits_to_read_from_next_buffer
                | self.read_bits(bits_to_read_from_next_buffer as usize)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::util::bitreader::BitReader;
    use hex;

    #[test]
    fn read_bits() {
        let data: Vec<u8> = hex::decode("7775626261206C756262612064756220647562").unwrap();
        let mut reader = BitReader::new(data.as_slice());
        assert_eq!(reader.read_bits(4), 0x7);
        assert_eq!(reader.read_bits(4), 0x7);
        assert_eq!(reader.read_bits(4), 0x7);
        assert_eq!(reader.read_bits(4), 0x5);
        assert_eq!(reader.read_bits(4), 0x6);
        assert_eq!(reader.read_bits(4), 0x2);
        assert_eq!(reader.read_bits(4), 0x6);
        // Should be at pos 28 by now, i.e. close to a boundary
        assert_eq!(reader.read_bits(8), 0x26);
        assert_eq!(reader.read_bits(8), 0x12);
        assert_eq!(reader.read_bits(8), 0x06);
        assert_eq!(reader.read_bits(8), 0xC7);
        assert_eq!(reader.read_bits(8), 0x56);
    }

    #[test]
    fn read_bits_test_end() {
        let data: Vec<u8> = hex::decode("777562").unwrap();
        let mut reader = BitReader::new(data.as_slice());
        assert_eq!(reader.read_bits(12), 0x777);
        assert_eq!(reader.read_bits(12), 0x562);
    }

    #[test]
    fn count_continuous_1s() {
        let data: Vec<u8> = hex::decode("FFF0100FF800").unwrap();
        let mut reader = BitReader::new(data.as_slice());
        assert_eq!(reader.count_continuous_1s(), 12);
        for _ in 0..100 {
            // Shouldn't increment the counter, so should be able to do this
            // infinitely
            assert_eq!(reader.count_continuous_1s(), 0);
        }
        assert_eq!(reader.read_bits(7), 0x00);
        assert_eq!(reader.count_continuous_1s(), 1);
        assert_eq!(reader.read_bits(8), 0x00);
        // This crosses the 32-bit boundary, should be 4 before + 5 after
        assert_eq!(reader.count_continuous_1s(), 9);
        // The last 11 bytes should be zeros
        assert_eq!(reader.read_bits(11), 0);
    }

    #[test]
    fn count_continuous_0s_and_1s() {
        let data: Vec<u8> = hex::decode("03FFF0E00FF800").unwrap();
        let mut reader = BitReader::new(data.as_slice());
        assert_eq!(reader.count_continuous_0s(), 6);
        assert_eq!(reader.count_continuous_1s(), 14);
        assert_eq!(reader.count_continuous_0s(), 4);
        assert_eq!(reader.count_continuous_1s(), 3);
        // This one crosses boundaries
        assert_eq!(reader.count_continuous_0s(), 9);
        assert_eq!(reader.read_bits(9), 0x1FF);
        // TODO: can't handle situations where we're at the end of file yet.
        //assert_eq!(reader.count_continuous_0s(), 11);
        // That's the end!
    }

    #[test]
    fn count_41_0s_across_3_bytes() {
        let data: Vec<u8> = hex::decode("FFFFFFF00000000007").unwrap();
        let mut reader = BitReader::new(data.as_slice());
        assert_eq!(reader.count_continuous_1s(), 28);
        assert_eq!(reader.count_continuous_0s(), 41);
        assert_eq!(reader.read_bits(3), 0x7);
    }
}
