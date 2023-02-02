![master](https://github.com/nineonine/linkerloader-rs/actions/workflows/rust.yml/badge.svg)

# linkerloader-rs

Simple Linker/Loader implementation in Rust (John R. Levine book exercises)

This project is purely an exercise to understand how linkers and loaders work.

Module objects consist entirely of lines of ASCII text. This makes it possible to create sample object files in a text editor, as well as making it easier to check the output files from the project.

**Supported features:**
* Object parsers
* Object linking
* Object (De)Serialization
* static libraries (both: directory and single file format)

**TODO:**
* cli interface
* Implement static library (dirlib) editing: add, delete, replace modules
* Implement static library (filelib) editing: add, delete, replace modules

**Build:**
```
cargo build
```

**Test:**
```
cargo test
```
