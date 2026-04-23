# Rust Patterns

Common patterns, anti-patterns, and decision frameworks for Rust development.

## Contents

- [Ownership and borrowing](#ownership-and-borrowing)
- [Smart pointer selection](#smart-pointer-selection)
- [Concurrency](#concurrency)
- [Error handling patterns](#error-handling-patterns)
- [Trait design](#trait-design)
- [Lifetime patterns](#lifetime-patterns)
- [Closures and iterators](#closures-and-iterators)
- [Structural patterns](#structural-patterns)
- [Anti-patterns](#anti-patterns)

## Ownership and borrowing

**Accept borrowed types for arguments.** Use `&str` over `&String`, `&[T]` over `&Vec<T>`, `&T` over `&Box<T>`. Deref coercion handles the conversion automatically, and this accepts a wider range of inputs.

**If you need ownership, take ownership.** Don't borrow and clone inside the function — let the caller decide whether to clone or move.

**If you don't need ownership, borrow.** Don't take ownership and drop. Let the caller keep their data.

**Use generics at API boundaries for flexibility:**

```rust
// Good: accepts &str, String, &String, PathBuf, etc.
fn open_file(path: impl AsRef<Path>) -> io::Result<File> { /* ... */ }

// Good: accepts Vec, slice, array, iterator output, etc.
fn process(items: impl IntoIterator<Item = i64>) { /* ... */ }
```

**Use `Cow<'_, str>` when a function sometimes needs to allocate and sometimes doesn't.** Avoids unconditional allocation while keeping a simple return type.

## Smart pointer selection

| Need | Type | Notes |
|------|------|-------|
| Heap allocation, single owner | `Box<T>` | Recursive types, trait objects, large stack values |
| Multiple owners, single-threaded | `Rc<T>` | Read-only sharing. Use `Rc::clone(&x)` to signal intent |
| Multiple owners + mutation, single-threaded | `Rc<RefCell<T>>` | Runtime borrow checking. Panics on violation |
| Shared state, multi-threaded | `Arc<Mutex<T>>` | Atomic ref counting + locking |
| Shared state, read-heavy multi-threaded | `Arc<RwLock<T>>` | Multiple readers OR one writer |
| Non-owning back-reference (break cycles) | `Weak<T>` | `upgrade()` returns `Option<Rc<T>>` |
| Thread communication | `mpsc::channel()` | Ownership transfer via messages |
| Simple atomic values across threads | `AtomicU64`, `AtomicBool`, etc. | Lock-free primitives |

**Key rules:**

- `Rc<T>` is single-threaded only. Use `Arc<T>` for multi-threading.
- `RefCell<T>` is single-threaded only. Use `Mutex<T>` for multi-threading.
- Use `Rc::clone(&x)` (not `x.clone()`) to signal you're incrementing the refcount, not deep-copying.
- Use `Weak<T>` for child-to-parent pointers in trees to prevent reference cycles.
- Access `Weak<T>` with `.upgrade()` — always check the `Option` since the referent may be dropped.
- `Box<T>` is required for recursive types to give the compiler a known size.
- Only smart pointers should implement `Deref`/`DerefMut`.

## Concurrency

**Message passing with channels:**

```rust
let (tx, rx) = mpsc::channel();
let tx2 = tx.clone(); // Multiple producers

thread::spawn(move || { tx.send(value).unwrap(); });
thread::spawn(move || { tx2.send(other).unwrap(); });

for received in rx { // Iterate until all senders drop
    process(received);
}
```

`send()` takes ownership — prevents use-after-send at compile time.

**Shared state with `Arc<Mutex<T>>`:**

```rust
let state = Arc::new(Mutex::new(initial_value));
for _ in 0..n {
    let state = Arc::clone(&state);
    thread::spawn(move || {
        let mut data = state.lock().unwrap();
        *data += 1;
    }); // MutexGuard drops here, releasing the lock
}
```

**Thread safety traits:**

- `Send`: ownership can transfer between threads. Almost all types. Exceptions: `Rc<T>`, raw pointers.
- `Sync`: safe to reference from multiple threads (`&T` is `Send`). Exceptions: `Rc<T>`, `RefCell<T>`, `Cell<T>`.
- Composed types are automatically `Send`/`Sync` if all fields are.

**Rules:**

- Always use `move` with `thread::spawn` closures to transfer ownership.
- Always `join()` spawned threads unless you explicitly want them detached.
- Acquire locks in consistent order across threads to prevent deadlocks.
- Prefer channels over shared state when the communication pattern is naturally message-based.
- Use `RwLock` over `Mutex` when reads vastly outnumber writes.

## Error handling patterns

**Enum-based errors (libraries):**

Define a crate-level error enum with `From` impls for each source error. This enables `?` propagation throughout the crate. See the error example in SKILL.md.

**`anyhow` (applications):**

Use `anyhow::Result` in binaries and application code where you need to propagate errors without matching on variants. Use `.context("what was being attempted")` to add context.

**`thiserror` (libraries):**

Derive `Error` and `Display` implementations to reduce boilerplate:

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid input for {field}: {reason}")]
    InvalidInput { field: &'static str, reason: String },
}
```

**Iterating over results:**

- `filter_map(|r| r.ok())` — silently ignore errors (rarely appropriate)
- `collect::<Result<Vec<_>, _>>()` — fail on first error (usually what you want)
- `partition(Result::is_ok)` — separate successes from failures

**When to `panic!`:**

- Unrecoverable invariant violations (bug in the program, not bad user input)
- `unreachable!()` for branches that should be impossible
- `unimplemented!()` / `todo!()` for stubs during development
- Test assertions

## Trait design

**Orphan rule:** You can only implement a trait on a type if either the trait or the type is local to your crate.

**Default implementations** can call other methods in the same trait, even unimplemented ones. You cannot call the default from an overriding implementation.

**When to use which trait bound syntax:**

| Pattern | Syntax | Use case |
|---------|--------|----------|
| `impl Trait` parameter | `fn f(x: &impl Summary)` | Simple, single trait requirement |
| Explicit trait bound | `fn f<T: Summary>(x: &T)` | Multiple params needing same concrete type |
| Multiple bounds | `T: Summary + Display` | Multiple trait requirements |
| `where` clause | `where T: Display + Clone` | Complex signatures, readability |
| `impl Trait` return | `-> impl Summary` | Hide concrete return type (single type only) |

**`impl Trait` return limitation:** Must be a single concrete type across all code paths. Use `Box<dyn Trait>` for multiple return types.

**Sealed traits** prevent downstream implementations while allowing you to add methods:

```rust
mod private { pub trait Sealed {} }
pub trait MyTrait: private::Sealed { /* ... */ }
impl private::Sealed for MyConcreteType {}
impl MyTrait for MyConcreteType { /* ... */ }
```

**Object safety:** If a trait may be used as `dyn Trait`, ensure methods don't use generics or return `Self`. Use `where Self: Sized` to exclude specific methods from the vtable.

## Lifetime patterns

**Elision rules (compiler applies automatically):**

1. Each reference parameter gets its own lifetime.
2. If exactly one input lifetime, it's assigned to all outputs.
3. If `&self`/`&mut self` is a parameter, its lifetime is assigned to all outputs.

**Structs with references:** The struct cannot outlive the data it borrows.

```rust
struct Excerpt<'a> {
    text: &'a str, // Instance cannot outlive the string it references
}
```

**`'static`:** All string literals have `'static` lifetime. Before applying `'static` to fix errors, verify the reference truly needs to live for the entire program — usually the error indicates a dangling reference that should be fixed at the source.

**Minimize lifetime annotations.** If the elision rules cover your case, don't annotate. Only add explicit lifetimes when the compiler requires them and you understand why.

## Closures and iterators

**Closure trait hierarchy:** `Fn` ⊂ `FnMut` ⊂ `FnOnce`.

| Trait | Calls | Captures | Example use |
|-------|-------|----------|-------------|
| `FnOnce` | Once | Moves values out | `thread::spawn`, consuming adapters |
| `FnMut` | Many | Mutably borrows | `sort_by_key`, stateful callbacks |
| `Fn` | Many | Immutable borrow or nothing | Concurrent calls, pure functions |

**Iterator patterns:**

- `iter()` borrows, yields `&T`. `iter_mut()` borrows mutably, yields `&mut T`. `into_iter()` takes ownership, yields `T`.
- Iterator adaptors (`map`, `filter`, `take`) are lazy — nothing happens until a consuming adaptor (`collect`, `sum`, `for_each`, `count`) is called.
- Iterators are zero-cost abstractions: they compile to equivalent hand-written loops.
- Prefer `iter().map().filter().collect()` chains over manual `for` loops with mutation.

**`Option` as iterator:** `Option` implements `IntoIterator`. Use with `.chain()` and `.extend()` for elegant optional element handling.

## Structural patterns

**Builder pattern** — for types with many optional parameters:

```rust
pub struct ServerBuilder {
    port: u16,
    max_connections: Option<usize>,
    timeout: Option<Duration>,
}

impl ServerBuilder {
    pub fn new(port: u16) -> Self {
        Self { port, max_connections: None, timeout: None }
    }
    pub fn max_connections(&mut self, n: usize) -> &mut Self {
        self.max_connections = Some(n); self
    }
    pub fn timeout(&mut self, d: Duration) -> &mut Self {
        self.timeout = Some(d); self
    }
    pub fn build(&self) -> Server { /* ... */ }
}
```

**Newtype pattern** — zero-cost type safety:

```rust
struct UserId(u64);
struct OrderId(u64);
// These can never be confused at compile time
fn get_order(user: UserId, order: OrderId) -> Order { /* ... */ }
```

**Compose structs for borrow splitting** — when the borrow checker prevents borrowing parts of a struct independently, break it into sub-structs:

```rust
// Instead of one large struct where borrowing `state` blocks borrowing `config`:
struct App {
    state: AppState,
    config: AppConfig,
}
// Now you can borrow state and config independently
fn update(state: &mut AppState, config: &AppConfig) { /* ... */ }
```

**`mem::take` and `mem::replace`** — transform owned data in place without cloning:

```rust
// Transform an enum variant without cloning
let old = std::mem::take(&mut self.field); // replaces with Default
let old = std::mem::replace(&mut self.field, new_value);
```

**RAII guards** — tie resource cleanup to scope via `Drop`:

```rust
struct TempFile { path: PathBuf }
impl Drop for TempFile {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.path); }
}
```

## Anti-patterns

**Cloning to satisfy the borrow checker.** If you're cloning to make the compiler happy, the ownership design is wrong. Restructure with `Rc`/`Arc`, split borrows, or redesign data flow. Run `cargo clippy` to catch unnecessary clones.

**`#![deny(warnings)]` in source code.** Breaks builds on compiler updates. Use `RUSTFLAGS="-D warnings"` in CI instead, or selectively deny specific stable lints.

**Stringly-typed APIs.** Use enums and newtypes instead of raw strings for structured data. `Status::Active` not `"active"`.

**`unwrap()` chains in production code.** Each `unwrap()` is a potential panic site. Use `?`, combinators, or explicit `match`.

**God structs.** If a struct has more than ~7 fields, it likely does too much. Split into focused sub-structs.

**Premature `dyn Trait`.** Use generics (static dispatch) by default. Only use `dyn Trait` when you actually need heterogeneous collections or to reduce code size.

**Manual `Drop` with fallible cleanup.** `Drop` must not panic. Provide a separate `close()` → `Result` method for fallible cleanup, and have `Drop` do best-effort (ignore/log errors).

**`static mut`.** Always unsound. Use `AtomicU64`, `Mutex`, `RwLock`, `LazyLock`, or `OnceLock` instead.

**Ignoring `must_use` warnings.** `Result` is `#[must_use]`. If you intentionally discard it, use `let _ = expr;` to signal intent.
