## Setup


Copy the dylib to the GUI app:

```
cargo build && cp target/debug/libblitzexport.dylib blitz_gui/build/macos/Build/Products/Debug/blitz_gui.app/Contents/Frameworks
```

## Profiling

```sh
# On the host machine
docker build -t rust-valgrind machine-images/valgrind && docker run -v `pwd`:/repo -it rust-valgrind
# Then within the container
cargo build --release && valgrind --tool=callgrind --dump-instr=yes --collect-jumps=yes --simulate-cache=yes target/release/blitz ROFL3343.raf
```
