use std::io;

// TODO: move to util submodule.
pub struct ByteCounter<W> {
    inner: W,
    count: usize,
}
// Implementation borrowed from here: https://stackoverflow.com/a/42189386/996592
impl<W> ByteCounter<W>
where
    W: io::Write,
{
    pub fn new(inner: W) -> Self {
        ByteCounter { inner, count: 0 }
    }

    pub fn bytes_written(&self) -> usize {
        self.count
    }
}

impl<W> io::Write for ByteCounter<W>
where
    W: io::Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let res = self.inner.write(buf);
        if let Ok(size) = res {
            self.count += size
        }
        res
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
