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

## Basic Usage

### Views & Navigation
There are 4 views, Services, Objects, Results and Call view.

- The Service view will show the services avaliable on your bus.
- The Object view will show object on the service selected.
- The Result view will show results from communicating with Dbus, i.e method calls or properties.
- The Call view will show the current active Method call or Property Set, with each of the arguments for a call being a text input.

There is a general navigation, and view specicic navigation

### General navigation

| Key | Action | Description |
| :--- | :--- | :--- |
| Arrows/h j k l | **Move Cursor** | Navigate menus, lists, and tables. |
| Tab | **Next view** | Switch focus to next view. |
| ? | **Help** | Show this help. |
| q | **Quit** | Quit application. |

---

### Services View navigation

| Key | Action | Description |
| :--- | :--- | :--- |
| Enter | **Select service** | Select service and get its objects. |

---

### Objects View navigation

| Key | Action | Description |
| :--- | :--- | :--- |
| Right/Left / h/j | **Expand/Collapse** | Expand/collapse a node. |
| Enter | **Call method / Get property** | Call a Method, or Get a property that is selected. |
| s | **Set property** | Set a property that is selected. |
| g | **Get property** | Get a property that is selected. |

---

### Call View navigation

| Key | Action | Description |
| :--- | :--- | :--- |
| i | **Enter insert mode** | Enter "Insert mode" which will allow you to insert text into the text fields. |
| Esc | **Enter normal mode** | Enter normal mode, allowing normal interactions such as navigation or calling the method. |
| Enter | **Call active** | Call the active method or set property based on the argument inputs given. |

---

## Parsing
The parsing for the arguments in the Call view are based on the DBus type signature from the argument in the method/property, you can see that type in the bottom of the text field.
The argument text field will turn green when the text can be parsed.

| D-Bus Type | Input Format | Separator | Example |
| :--- | :--- | :--- | :--- |
| **Basic Types** | **Value** (e.g., `5`, `true`, `"path"`) | N/A | `5` for `i`, `true` for `b` |
| **Array** (`a...`) | Elements enclosed by **`[` and `]`** | Comma (`,`) | `["first", "second"]` |
| **Structure** (`(...)`) | Elements enclosed by **`(` and `)`** | Comma (`,`) | `("text", 123)` for `(su)` |
| **Dictionary** (`a{...}`) | Key-value pairs enclosed by **`{` and `}`** | Key/Value: Colon (`:`) <br> Pairs: Comma (`,`) | `{"key": "value", "key2": "value2"}` |
| **Variant** (`v`) | **Signature** followed by `->` and **Value** | `->` | `"u"->5` (means `u32` with value `5`) |

-----

### Basic Types

| Signature | Example Input | D-Bus Value |
| :--- | :--- | :--- |
| `y` (byte/u8) | `255` | 255 |
| `i` (integer/i32) | `-42` | -42 |
| `d` (double/f64) | `3.1415` | 3.1415 |
| `b` (boolean) | `true` or `false` | True or False |
| `s` (string) | `"Hello World"` | "Hello World" |
| `o` (object path) | `"/org/example/path"` | ObjectPath |
| `g` (signature) | `"as"` | Signature value |

-----

### Complex Type Examples

| Signature | Example Input | Breakdown |
| :--- | :--- | :--- |
| `a{si}` | `{"count": 5, "max": 10}` | A dictionary (array of entries) where keys are **string** (`s`) and values are **integer** (`i`). |
| `(ssu)` | `("user_name", "john", 42)` | A structure with three elements: **string**, **string**, **u32**. |
| `a(is)` | `[(1, "a"), (2, "b")]` | An array of structures, where each structure has an **integer** and a **string**. |
| `v` | `"as"->["one", "two"]` | A variant containing an array of strings (`as`) with the values `["one", "two"]`. |

**Note:** File Descriptors (`h`) cannot be parsed from the input string and will result in an error.

---


### ArchLinux
[AUR dtui package](https://aur.archlinux.org/packages/dtui)
