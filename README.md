# dterm
A small TUI for d-termining the state of you dbus.
This will show you the current services running and allows you to introspect that service.

Use Tab to change between the view, arrow keys to navigate the list and tree, and enter to get objects from a service.
![Example](/images/dterm.png)

## Build
To build install Rust and cargo, then run build
```
cargo build
```

To run from cargo
```
cargo run --bin dterm
```