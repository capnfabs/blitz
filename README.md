## Setup

The Xcode Project automatically links the Dylib produced by `cargo build` for the `blitzexport` crate. To build the Rust component:

```sh
cargo build
```

To build the OSX component, use XCode.

## Profiling

```sh
# On the host machine
docker build -t rust-valgrind machine-images/valgrind && docker run -v `pwd`:/repo -it rust-valgrind
# Then within the container
cargo build --release && valgrind --tool=callgrind --dump-instr=yes --collect-jumps=yes --simulate-cache=yes target/release/blitz ROFL3343.raf
```
