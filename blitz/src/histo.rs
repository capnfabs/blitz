use std::iter::Enumerate;

pub struct Histo {
    data: Vec<usize>,
    found_min: u16,
    found_max: u16,
    total: usize,
}

impl Histo {
    pub fn new() -> Self {
        let data = vec![0usize; std::u16::MAX as usize + 1];
        Histo {
            data,
            found_min: std::u16::MAX,
            found_max: 0,
            total: 0,
        }
    }

    pub fn from_iter(iter: impl Iterator<Item = u16>) -> Self {
        let mut h = Self::new();
        for v in iter {
            h.add(v);
        }
        h
    }

    pub fn add(&mut self, item: u16) {
        if item > self.found_max {
            self.found_max = item;
        }
        if item < self.found_min {
            self.found_min = item;
        }
        self.data[item as usize] += 1;
        self.total += 1;
    }

    pub fn view_clipped(&self, buckets: usize) -> HistoView {
        let range = (self.found_max - self.found_min) as usize;
        // round up to ensure the entire range is included.
        let bucket_width = (range + buckets - 1) / buckets;
        let data = self.data[self.found_min as usize..]
            .chunks(bucket_width)
            .map(|chunk| chunk.iter().sum())
            .collect();
        HistoView {
            data,
            min: self.found_min,
            max: self.found_max,
            bucket_width: bucket_width as u16,
        }
    }
}

pub struct HistoView {
    data: Vec<usize>,
    min: u16,
    max: u16,
    bucket_width: u16,
}

impl HistoView {
    pub fn iter(&self) -> ViewIter {
        ViewIter {
            iter: self.data.iter().enumerate(),
            min: self.min,
            max: self.max,
            bucket_width: self.bucket_width,
        }
    }
}

pub struct ViewIter<'a> {
    iter: Enumerate<core::slice::Iter<'a, usize>>,
    min: u16,
    max: u16,
    bucket_width: u16,
}

pub struct Bucket {
    pub min: u16,
    pub max: u16,
    pub count: usize,
}

impl<'a> Iterator for ViewIter<'a> {
    type Item = Bucket;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(idx, count)| Bucket {
            min: self.min + self.bucket_width * idx as u16,
            // There are combinations of values for which this will die.
            max: self.max + self.bucket_width * (idx + 1) as u16,
            count: *count,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> ExactSizeIterator for ViewIter<'a> {}
