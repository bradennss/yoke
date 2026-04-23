# Rust Code Quality Checklist

Run through relevant sections before finalizing Rust code changes.

## Correctness

- [ ] No `unwrap()` or `expect()` in production code paths (tests are fine)
- [ ] Error types implement `std::error::Error`, `Display`, `Debug`, `Send`, `Sync`
- [ ] `From` impls exist for all error variants to enable `?` propagation
- [ ] No `static mut` usage — use `Atomic*`, `Mutex`, `RwLock`, `OnceLock`, `LazyLock`
- [ ] Destructors (`Drop`) never panic — fallible cleanup uses a separate `close()` method
- [ ] `unsafe` blocks have a `// SAFETY:` comment explaining why invariants hold
- [ ] `must_use` values are not silently discarded (use `let _ =` if intentional)

## Naming

- [ ] Types, Traits, Variants: `UpperCamelCase` (acronyms as one word: `Uuid` not `UUID`)
- [ ] Functions, Methods, Modules: `snake_case`
- [ ] Constants, Statics: `SCREAMING_SNAKE_CASE`
- [ ] Getters: no `get_` prefix (`fn name(&self)` not `fn get_name(&self)`)
- [ ] Conversions: `as_` (free/borrowed), `to_` (expensive/owned), `into_` (consuming)
- [ ] Iterators: `iter()`, `iter_mut()`, `into_iter()`
- [ ] Constructors: `new()`, `with_*()`, `from_*()`

## Type design

- [ ] Newtypes used to distinguish values sharing an underlying type
- [ ] Enums used over booleans for function parameters with semantic meaning
- [ ] Builder pattern used for types with multiple optional parameters
- [ ] `bitflags` used for sets of flags (not enums)
- [ ] Struct fields are private with methods for access (unless there's a specific reason)
- [ ] `#[non_exhaustive]` on public enums/structs that may gain variants/fields

## Traits

- [ ] Common traits implemented eagerly: `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Default`
- [ ] `Display` implemented for user-facing types
- [ ] `From`/`TryFrom` implemented for conversions (never `Into`/`TryInto` directly)
- [ ] `FromIterator` and `Extend` implemented for collection types
- [ ] Serde `Serialize`/`Deserialize` gated behind a `"serde"` feature
- [ ] `Send`/`Sync` bounds verified for types with raw pointers
- [ ] No unnecessary trait bounds on data structures (bounds go on impls, not struct defs)

## API design

- [ ] Functions accept borrowed types: `&str` not `&String`, `&[T]` not `&Vec<T>`
- [ ] Generics used at API boundaries: `impl AsRef<Path>`, `impl IntoIterator`
- [ ] Functions return owned data, not out-parameters (except buffer-reuse cases)
- [ ] Conversions live on the most specific type involved
- [ ] Functions with a clear receiver are methods (not free functions)
- [ ] Intermediate results exposed to avoid duplicate work
- [ ] Reader/writer functions take `R: Read` / `W: Write` by value

## Error handling

- [ ] Library errors: crate-level error enum with `From` impls
- [ ] Application errors: `anyhow::Result` or equivalent
- [ ] Error messages: lowercase, no trailing punctuation
- [ ] `panic!` only for programming errors (invariant violations), never for user input
- [ ] `()` never used as an error type

## Documentation

- [ ] Crate root has module-level documentation
- [ ] Public items have doc comments with examples
- [ ] Doc examples use `?` (not `unwrap()` or `try!`)
- [ ] Functions document `# Errors`, `# Panics`, `# Safety` sections where applicable
- [ ] `Cargo.toml` has `description`, `license`, `repository`

## Testing

- [ ] Unit tests in `#[cfg(test)] mod tests` alongside source
- [ ] Integration tests in `tests/` for public API behavior
- [ ] Each test verifies one behavior with a descriptive name
- [ ] Error conditions tested (not just happy paths)
- [ ] Custom types in assertions derive `PartialEq` and `Debug`
- [ ] Shared test utilities in `tests/common/mod.rs`

## Structure

- [ ] `main.rs` is thin — logic lives in `lib.rs`
- [ ] One concept per module
- [ ] `pub use` creates a clean public API
- [ ] No circular dependencies between modules

## Validation

- [ ] `cargo fmt` passes
- [ ] `cargo check` passes
- [ ] `cargo clippy` passes (or warnings are addressed)
- [ ] `cargo test` passes
