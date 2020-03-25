use itertools::Itertools;
use ndarray::{Array2, ArrayView2, Axis};
use rayon::prelude::*;

pub trait SingleInputSingleOutput<In, Out = In>: Fn(usize, usize, In) -> Out + Send + Sync {}
impl<T, In, Out> SingleInputSingleOutput<In, Out> for T where
    T: Fn(usize, usize, In) -> Out + Send + Sync
{
}

pub trait RandomAccessInputSingleOutput<In, Out = In>:
    Fn(usize, usize, &ArrayView2<In>) -> Out + Sync + Send
{
}
impl<T, In, Out> RandomAccessInputSingleOutput<In, Out> for T where
    T: Fn(usize, usize, &ArrayView2<In>) -> Out + Sync + Send
{
}

pub fn par_index_map_siso<In: Sync + Copy, Out: Copy + Sync + Send>(
    data: &ArrayView2<In>,
    func: impl SingleInputSingleOutput<In, Out>,
) -> Array2<Out> {
    // We'll initialize all values by zipping this with the other iterator.
    let mut out = unsafe { Array2::uninitialized(data.raw_dim()) };

    // TODO: make this a constant / controlled by context or something
    let chunks = 8 * 4;
    let lines_per_chunk = data.len_of(Axis(0)) / chunks;

    let input_chunks = data
        .axis_chunks_iter(Axis(1), lines_per_chunk)
        .collect_vec();

    let output_chunks = out.axis_chunks_iter_mut(Axis(1), lines_per_chunk);

    input_chunks
        .par_iter()
        .zip(output_chunks)
        .enumerate()
        .for_each(|(chunk_idx, (input_chunk, mut output_chunk))| {
            for (&input, ((x, y), out_ref)) in
                input_chunk.iter().zip(output_chunk.indexed_iter_mut())
            {
                *out_ref = func(x, y + chunk_idx * lines_per_chunk, input);
            }
        });

    out
}

pub fn par_index_map_raiso<In: Sync + Copy, Out: Copy + Sync + Send>(
    data: &ArrayView2<In>,
    func: impl RandomAccessInputSingleOutput<In, Out>,
) -> Array2<Out> {
    // We'll initialize all values by zipping this with the other iterator.
    let mut out = unsafe { Array2::uninitialized(data.raw_dim()) };

    // TODO: make this a constant / controlled by context or something
    let chunks = 8 * 4;
    let lines_per_chunk = data.len_of(Axis(0)) / chunks;

    let mut output_chunks = out
        .axis_chunks_iter_mut(Axis(1), lines_per_chunk)
        .collect_vec();

    output_chunks
        .par_iter_mut()
        .enumerate()
        .for_each(|(chunk_idx, output_chunk)| {
            for ((x, y), out_ref) in output_chunk.indexed_iter_mut() {
                *out_ref = func(x, y + chunk_idx * lines_per_chunk, data);
            }
        });

    out
}

// Ok, here's the thought process:

// - A _step_ should take a source image, and an output slice, and be able to populate the output slice from the source.
// - Can make a wrapper to convert image + coord -> pixels into image -> output slice for convenience
// - Should probably be generic.
// - Something to watch out for -- we probably want to make sure that we're not allowing old data through, but we can't really guarantee that if we're trying to minimise allocations.
// - Maybe we shouldn't bother with the "two buffer" thing to start -- just rely on the memory manager, allocate new memory and then optimise later if it's slow.
// This can support cropping, I think.
// Different patterns that I might want:
// Single pixel:
// - Input: each pixel
// - Output: write to same pixel
// Multi-source, single dest:
// - Each output pixel is expressible as a function of multiple source pixels, can be accessed arbitrarily / with relative position
// ?? Multi-source, multi-dest? Probably useful for adaptive algorithms, etc.

// What I would like is
/*
fn example_input() {
    run_steps!(
        image, // ---
        devignette, black_sub, to_float, demosaic, white_bal, clamp, to_srgb,
    )
}
// And this should do something like:
fn example_output() {
    let thing = image
        .indexed_iter_mut()
        .par_iter_mut()
        .map(devignette)
        .map(black_sub)
        .map(to_float)
        .collect();
    let thing = demosaic(thing)
        .indexed_iter_mut()
        .par_iter_mut()
        .map(white_bal)
        .map(clamp)
        .map(to_srgb)
        .collect();
}
*/
