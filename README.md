# Ytterbium - Rust Synthesizer

...

## Notes on Rust

- fixed-size array initialization: `let name: [type, size] = [value, size] or [1, 2, ... times size]`, example: `let name: [i32, 5] = [1,2,4,8,16]`
- `!try` macro:

    ```rust
    !try(e)  // expands to:

    match $e {
        Ok(e) => e,
        Err(e) => return Err(e)
    }
    ```
- the unit type `()` can be used like pythons `pass`

### Error Handling

- use `Result` as return type
- match against `Result` options `Ok(t), Err(e)`
- use custom Result types: [...](http://www.hydrocodedesign.com/2014/05/28/practicality-with-rust-error-handling/)