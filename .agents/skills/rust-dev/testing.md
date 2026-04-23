# Rust Testing

Test organization, assertion patterns, and best practices.

## Contents

- [Test organization](#test-organization)
- [Writing tests](#writing-tests)
- [Assertions](#assertions)
- [Testing patterns](#testing-patterns)
- [Running tests](#running-tests)

## Test organization

### Unit tests

Place alongside the code they test. Can access private items.

```rust
// src/parser.rs
fn parse_internal(input: &str) -> Result<Ast, ParseError> { /* ... */ }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_input() {
        assert!(parse_internal("").is_err());
    }
}
```

`#[cfg(test)]` ensures test code is excluded from release builds.

### Integration tests

Live in `tests/` at the project root. Each `.rs` file compiles as a separate crate. Can only test the public API.

```rust
// tests/parsing.rs
use mycrate::parse;

#[test]
fn parse_valid_document() {
    let result = parse("valid input");
    assert!(result.is_ok());
}
```

### Shared test utilities

Place in `tests/common/mod.rs` — NOT `tests/common.rs` (which would be treated as its own test crate).

```rust
// tests/common/mod.rs
pub fn setup() -> TestContext { /* ... */ }

// tests/integration.rs
mod common;

#[test]
fn test_with_setup() {
    let ctx = common::setup();
    // ...
}
```

### Doc tests

Code examples in doc comments are compiled and run as tests. Use the `?` operator pattern:

```rust
/// Parses the input string.
///
/// ```
/// # use mycrate::parse;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let result = parse("hello")?;
/// assert_eq!(result.value(), "hello");
/// #     Ok(())
/// # }
/// ```
pub fn parse(input: &str) -> Result<Parsed, Error> { /* ... */ }
```

Lines prefixed with `#` are compiled but hidden from rendered docs.

### Binary vs library crate testing

Integration tests can only test library crates (`lib.rs`). Keep logic in `lib.rs` with a thin `main.rs` wrapper. Binary-only crates cannot have integration tests.

### Execution order

Unit tests → Integration tests → Doc tests. If a section fails, subsequent sections may not run.

## Writing tests

### Result-returning tests

Use `Result` return to enable `?` in tests:

```rust
#[test]
fn parse_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_str("key=value")?;
    assert_eq!(config.get("key"), Some("value"));
    Ok(())
}
```

### Expected panics

```rust
#[test]
#[should_panic(expected = "index out of bounds")]
fn index_beyond_length() {
    let v = vec![1, 2, 3];
    let _ = v[99];
}
```

Use `expected = "substring"` to verify the panic message. Cannot combine with `Result`-returning tests.

### Ignored tests

```rust
#[test]
#[ignore]
fn expensive_integration_test() { /* ... */ }
```

Run with `cargo test -- --ignored`. Run all including ignored: `cargo test -- --include-ignored`.

### Derive requirements

Custom types in `assert_eq!`/`assert_ne!` must implement `PartialEq` and `Debug`:

```rust
#[derive(Debug, PartialEq)]
struct Point { x: f64, y: f64 }
```

## Assertions

| Macro | Purpose | Required traits |
|-------|---------|----------------|
| `assert!(expr)` | Boolean truth | None (must be `bool`) |
| `assert_eq!(a, b)` | Equality | `PartialEq + Debug` |
| `assert_ne!(a, b)` | Inequality | `PartialEq + Debug` |
| `assert!(result.is_ok())` | Result is Ok | None |
| `assert!(result.is_err())` | Result is Err | None |
| `assert_matches!(val, pattern)` | Pattern match (nightly/crate) | `Debug` |

All assertion macros accept optional format string arguments:

```rust
assert_eq!(actual, expected, "expected {expected} for input {input:?}");
```

## Testing patterns

### Test one thing per test

Each test should verify a single behavior. Name tests for the behavior: `fn rejects_empty_input()` not `fn test1()`.

### Arrange-Act-Assert

```rust
#[test]
fn calculates_total_with_discount() {
    // Arrange
    let cart = Cart::new();
    let cart = cart.add(Item::new("widget", 100));
    let discount = Discount::percentage(10);

    // Act
    let total = cart.total_with_discount(&discount);

    // Assert
    assert_eq!(total, 90);
}
```

### Testing error conditions

Verify both the error variant and its content:

```rust
#[test]
fn rejects_negative_quantity() {
    let result = Order::new("item", -1);
    match result {
        Err(OrderError::InvalidQuantity(q)) => assert_eq!(q, -1),
        other => panic!("expected InvalidQuantity, got {other:?}"),
    }
}
```

### Test helpers

Keep setup code in helper functions. Prefix with underscore or place in a `helpers` module to avoid confusion with test functions.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> Config {
        Config::builder().port(8080).timeout(Duration::from_secs(30)).build()
    }

    #[test]
    fn config_has_correct_port() {
        let config = sample_config();
        assert_eq!(config.port(), 8080);
    }
}
```

### Property-based testing

Use the `proptest` crate when input space is large and manual examples are insufficient:

```rust
proptest! {
    #[test]
    fn roundtrip_serialize(val: MyType) {
        let bytes = serialize(&val);
        let decoded = deserialize(&bytes)?;
        prop_assert_eq!(val, decoded);
    }
}
```

### Compile-time trait tests

Verify `Send`/`Sync` without running anything:

```rust
#[test]
fn types_are_send_and_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<MyType>();
    assert_sync::<MyType>();
}
```

## Running tests

| Command | Effect |
|---------|--------|
| `cargo test` | Run all tests (unit, integration, doc) in parallel |
| `cargo test name` | Filter tests by substring match on name |
| `cargo test -- --test-threads=1` | Run sequentially (for tests sharing state) |
| `cargo test -- --show-output` | Show stdout from passing tests |
| `cargo test --test integration` | Run specific integration test file |
| `cargo test -- --ignored` | Run only `#[ignore]` tests |
| `cargo test -- --include-ignored` | Run all tests including ignored |
| `cargo test --lib` | Unit tests only |
| `cargo test --doc` | Doc tests only |
| `cargo test -p crate_name` | Tests for a specific workspace crate |
