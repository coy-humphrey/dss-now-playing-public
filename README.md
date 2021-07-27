# dss-now-playing

## Build steps
This project uses vcpkg to install SDL dependencies.

To install SDL dependencies and build, please run:

```bash
cargo install cargo-vcpkg # Needs to be run only once globally
cargo vcpkg build # Needs to be run once per project, or again if project vcpkg dependencies change
cargo build # Builds non-vcpkg source as usual
```

## Help Info

```
USAGE:
    dss-now-playing.exe [FLAGS] <font-path>

ARGS:
    <font-path>    TTF font file for displaying text

FLAGS:
    -b, --bounded     Limits to a single active download
    -h, --help        Prints help information
    -s, --slow        Slows image downloads to show off asynchronous behavior
    -t, --threaded    Use multiple threads
    -V, --version     Prints version information
```

Be sure to specify a font when starting, for example: `cargo run C:\Windows\Fonts\times.ttf` on Windows.