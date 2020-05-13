use std::iter::Enumerate;

pub struct Histo {
    data: Vec<usize>,
    found_min: f32,
    found_max: f32,
    total: usize,
}

impl Histo {
    pub fn new() -> Self {
        // Default to 65k
        Self::with_buckets(1 << 16)
    }

    pub fn with_buckets(num_buckets: usize) -> Self {
        let data = vec![0usize; num_buckets];
        Histo {
            data,
            found_min: std::f32::MIN,
            found_max: std::f32::MAX,
            total: 0,
        }
    }

    pub fn from_iter(iter: impl Iterator<Item = f32>, num_buckets: usize) -> Self {
        let mut h = Self::with_buckets(num_buckets);
        for v in iter {
            h.add(v);
        }
        h
    }

    pub fn add(&mut self, item: f32) {
        if item > self.found_max {
            self.found_max = item;
        }
        if item < self.found_min {
            self.found_min = item;
        }
        let mut index = (item * (self.data.len() - 1) as f32) as usize;
        if index >= self.data.len() {
            index = self.data.len() - 1;
        }
        self.data[index] += 1;
        self.total += 1;
    }

    pub fn iter(&self) -> ViewIter {
        let bucket_width = 1.0 / (self.data.len() + 1) as f32;
        ViewIter {
            iter: self.data.iter().enumerate(),
            min: self.found_min,
            max: self.found_max,
            bucket_width,
        }
    }
}

pub struct HistoView {
    data: Vec<usize>,
    min: f32,
    max: f32,
    bucket_width: f32,
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
    min: f32,
    max: f32,
    bucket_width: f32,
}

pub struct Bucket {
    pub min: f32,
    pub max: f32,
    pub count: usize,
}

impl<'a> Iterator for ViewIter<'a> {
    type Item = Bucket;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(idx, count)| Bucket {
            min: self.min + self.bucket_width * idx as f32,
            // f32here are combinations of values for which this will die.
            max: self.max + self.bucket_width * (idx + 1) as f32,
            count: *count,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> ExactSizeIterator for ViewIter<'a> {}
