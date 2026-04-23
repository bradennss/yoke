---
name: rust-dev
description: Writes and maintains Rust code that is readable, maintainable, testable, performant, and idiomatic. Applies official Rust API guidelines and established patterns. Use when writing, reviewing, refactoring, or debugging Rust code.
when_to_use: When working with .rs files, Cargo.toml, Rust projects, or when the user mentions Rust, cargo, clippy, or rustc.
effort: max
---

# Rust Development

Write Rust code that is readable, maintainable, testable, performant, and idiomatic — in that priority order.

## Approach to changes

When making changes or fixing bugs, always examine the larger scope. Do not take shortcuts. If the right fix requires structural changes — moving code between modules, redesigning a type hierarchy, refactoring an error type, splitting a function — make those changes. A quick patch that obscures intent or adds technical debt is not acceptable.

Before modifying code:

1. **Understand the context.** Read surrounding code, the module's purpose, and how it integrates with the rest of the system.
2. **Identify the root cause.** For bugs, trace to the underlying issue. A symptom fix that leaves the root cause intact will resurface.
3. **Assess the blast radius.** Determine what depends on the code you're changing. Check callers, trait implementors, and downstream consumers.
4. **Make the structural change.** If the code's current structure is the reason the bug exists or the feature is hard to add, fix the structure first. Then the change becomes natural.
5. **Validate.** Run `cargo check`, `cargo clippy`, and `cargo test` after changes.

## Code quality standards

### Readability

- Name things for what they represent, not how they're implemented. `user_count` not `len`, `is_valid` not `check`.
- Favor explicit over clever. A clear `match` is better than a chain of combinators that requires mental unpacking.
- Keep functions short and focused. If a function does two things, split it.
- Group related items together. Put the public API at the top of a module.
- Use `rustfmt` defaults. Do not fight the formatter.

### Maintainability

- Prefer strong types over primitive obsession. Use newtypes (`struct Miles(f64)`) to distinguish values that share an underlying type.
- Use enums over booleans for function parameters. `Widget::new(Size::Small, Shape::Round)` not `Widget::new(true, false)`.
- Keep struct fields private. Expose behavior through methods. This preserves the ability to change representation without breaking callers.
- Use the builder pattern for types with many optional configuration parameters.
- Use `#[non_exhaustive]` on public enums and structs to allow future additions without breaking changes.

### Testability

- Design for testability from the start. Accept traits instead of concrete types where testing benefits.
- Keep business logic separate from I/O. Functions that compute should not also read files or make network calls.
- Return `Result` from functions that can fail — this makes them testable without catching panics.
- See [testing.md](testing.md) for test organization and patterns.

### Performance

- Prefer iterators over manual loops — they are zero-cost abstractions and often optimize better.
- Accept borrowed types in function arguments: `&str` over `&String`, `&[T]` over `&Vec<T>`, `&T` over `&Box<T>`.
- Use generics (`impl Into<String>`, `impl AsRef<Path>`) at API boundaries to avoid unnecessary allocations by callers.
- Do not optimize prematurely. Write clear code first, then profile and optimize hot paths.
- Trust `--release` builds. Benchmarks in debug mode are meaningless.

### Idiomatic Rust

- Follow the official Rust API Guidelines. See [api-guidelines.md](api-guidelines.md) for the complete reference.
- Use `?` for error propagation. Never `unwrap()` in library or production code paths.
- Implement standard traits eagerly: `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Default`, `Display` where appropriate.
- Use `derive` macros for standard trait implementations. Only write manual impls when custom behavior is needed.
- Destructure in `match` and `let` bindings to extract values clearly.
- Use `clippy` and treat its suggestions as guidance — they encode community consensus.

## Naming conventions

Follow RFC 430 strictly:

| Item                         | Convention                                | Example                        |
| ---------------------------- | ----------------------------------------- | ------------------------------ |
| Types, Traits, Enum variants | `UpperCamelCase`                          | `HttpResponse`, `IntoIterator` |
| Functions, Methods, Modules  | `snake_case`                              | `parse_header`, `into_inner`   |
| Constants, Statics           | `SCREAMING_SNAKE_CASE`                    | `MAX_RETRIES`, `DEFAULT_PORT`  |
| Type parameters              | Single uppercase or short CamelCase       | `T`, `K`, `V`, `Item`          |
| Lifetimes                    | Short lowercase                           | `'a`, `'de`, `'src`            |
| Crate names                  | `snake_case` (no `-rs` or `-rust` suffix) | `serde_json`                   |

Acronyms count as one word in CamelCase: `Uuid` not `UUID`, `HttpUrl` not `HTTPUrl`. In snake_case, acronyms are lowered: `is_xid_start`.

**Conversion methods:**

| Prefix  | Cost      | Ownership       | Example                        |
| ------- | --------- | --------------- | ------------------------------ |
| `as_`   | Free      | `&self` → `&T`  | `as_bytes()`, `as_str()`       |
| `to_`   | Expensive | `&self` → owned | `to_string()`, `to_vec()`      |
| `into_` | Variable  | `self` → owned  | `into_inner()`, `into_bytes()` |

**Getters:** No `get_` prefix. A field `name` has getter `fn name(&self)` and `fn name_mut(&mut self)`.

**Iterators:** `iter()` → `&T`, `iter_mut()` → `&mut T`, `into_iter()` → `T`.

**Constructors:** `new()` for primary, `with_capacity()` / `from_parts()` for secondary. I/O types may use domain names: `File::open`, `TcpStream::connect`.

## Error handling

1. **Library code:** Define a crate-level error enum. Implement `std::error::Error`, `Display`, `Debug`. Must be `Send + Sync`. Use `From` impls for `?` conversion.

```rust
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Parse(ParseError),
    InvalidInput { field: &'static str, reason: String },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io error: {e}"),
            Error::Parse(e) => write!(f, "parse error: {e}"),
            Error::InvalidInput { field, reason } => {
                write!(f, "invalid {field}: {reason}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Parse(e) => Some(e),
            Error::InvalidInput { .. } => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self { Error::Io(e) }
}
```

2. **Application code:** `anyhow::Result` is appropriate when you don't need to match on error variants.

3. **Rules:**
   - Never use `()` as an error type — it implements neither `Error` nor `Display`
   - Never `unwrap()` or `expect()` in production code paths (tests and provably infallible cases are fine)
   - Error messages from `Display`: lowercase, no trailing punctuation ("invalid input" not "Invalid input.")
   - `panic!` is for programming errors only — unrecoverable invariant violations, not user input or I/O failures
   - Implement `From<SourceError>` for each wrapped error to enable `?` propagation

## Type design decisions

| Situation                                    | Pattern                             |
| -------------------------------------------- | ----------------------------------- |
| Distinguish values with same underlying type | Newtype: `struct Miles(f64)`        |
| Multiple optional config parameters          | Builder pattern                     |
| Set of on/off flags                          | `bitflags` crate                    |
| Single-threaded shared ownership             | `Rc<T>` or `Rc<RefCell<T>>`         |
| Multi-threaded shared state                  | `Arc<Mutex<T>>` or `Arc<RwLock<T>>` |
| Recursive data structures                    | `Box<T>` for indirection            |
| Heterogeneous collections                    | `Box<dyn Trait>` or `&dyn Trait`    |
| Non-owning back-references (prevent cycles)  | `Weak<T>`                           |
| Thread communication                         | `mpsc::channel`                     |

See [patterns.md](patterns.md) for detailed guidance on each pattern.

## Module organization

- Crate root (`lib.rs` / `main.rs`) contains crate-level docs and re-exports.
- One concept per module. A module named `auth` contains authentication logic, not utilities.
- Use `pub use` to create a clean public API that doesn't mirror internal structure.
- Keep `main.rs` thin — parse args and call into library code. Integration tests can only test library crates.
- Place integration tests in `tests/`, unit tests in `#[cfg(test)] mod tests` alongside the code.
- Shared test utilities go in `tests/common/mod.rs` (not `tests/common.rs`).

## Validation workflow

After any change, run in order:

```
cargo fmt             # Format first
cargo check           # Type checking
cargo clippy          # Lint checking
cargo test            # All tests
```

Fix issues in this order: compilation errors → clippy warnings → test failures → formatting.

## Reference

- **API Guidelines**: [api-guidelines.md](api-guidelines.md) — Official Rust API Guidelines (40 rules covering naming, interoperability, type safety, documentation, and more)
- **Patterns**: [patterns.md](patterns.md) — Ownership patterns, concurrency patterns, smart pointer selection, and common anti-patterns
- **Testing**: [testing.md](testing.md) — Test organization, assertion patterns, and testing strategies
- **Checklist**: [checklist.md](checklist.md) — Quality review checklist for Rust code
