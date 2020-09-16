use hdrhistogram;
use itertools::Itertools;

pub struct Histogram(u8);

pub trait ToHistogram {
    fn histogram(&self) -> Histogram;
}

impl ToHistogram for image::RgbImage {
    fn histogram(&self) -> Histogram {
        // One per channel. TODO: make this generic somehow
        let mut hists = (0..3)
            .map(|_| hdrhistogram::Histogram::<u32>::new_with_bounds(1, 256, 3).unwrap())
            .collect_vec();

        for val in self.pixels() {
            for (hist, channel_val) in hists.iter_mut().zip_eq(val.0.iter().copied()) {
                hist.record(channel_val as u64).unwrap();
            }
        }
        for (chan, hist) in hists.iter().enumerate() {
            println!("Channel {}", chan);
            println!("Min: {}", hist.min());
            println!("05%: {}", hist.value_at_quantile(0.05));
            println!("25%: {}", hist.value_at_quantile(0.25));
            println!("50%: {}", hist.value_at_quantile(0.50));
            println!("75%: {}", hist.value_at_quantile(0.75));
            println!("95%: {}", hist.value_at_quantile(0.95));
            println!("Max: {}", hist.max());
            println!("---------");
        }

        Histogram(9)
    }
}
