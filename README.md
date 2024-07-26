# dtui
[![build](https://github.com/Troels51/dtui/actions/workflows/build.yml/badge.svg)](https://github.com/Troels51/dtui/actions/workflows/build.yml)
![Crates.io Version](https://img.shields.io/crates/v/dtui?link=https%3A%2F%2Fcrates.io%2Fcrates%2Fdtui)

A small TUI for d-termining the state of your dbus.
It will show you the current services running and allow you to introspect objects and their interfaces in those services

![Example](/images/dtui.png)

## Build
### From Source
To build install Rust and cargo, then run build
```sh
cargo build
```

To run from cargo
```sh
cargo run --bin dtui
```

### ArchLinux
[AUR dtui package](https://aur.archlinux.org/packages/dtui)
