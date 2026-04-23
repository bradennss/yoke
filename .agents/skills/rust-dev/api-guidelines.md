# Rust API Guidelines

Official guidelines from https://rust-lang.github.io/api-guidelines/. Each rule has a code (e.g., C-CASE) for reference.

## Contents

- [Naming](#naming)
- [Interoperability](#interoperability)
- [Macros](#macros)
- [Documentation](#documentation)
- [Predictability](#predictability)
- [Flexibility](#flexibility)
- [Type safety](#type-safety)
- [Dependability](#dependability)
- [Debuggability](#debuggability)
- [Future proofing](#future-proofing)
- [Necessities](#necessities)

## Naming

**C-CASE: Casing conforms to RFC 430.**
Types/Traits/Variants: `UpperCamelCase`. Functions/Methods/Modules: `snake_case`. Constants/Statics: `SCREAMING_SNAKE_CASE`. Type params: concise uppercase (`T`). Lifetimes: short lowercase (`'a`). Acronyms are one word: `Uuid` not `UUID`, `Usize` not `USize`. In snake_case, a "word" should never be a single letter unless it is the last word: `btree_map` not `b_tree_map`. No `-rs` or `-rust` crate suffixes.

**C-CONV: Ad-hoc conversions follow `as_`/`to_`/`into_` conventions.**
`as_`: free, borrowed→borrowed. `to_`: expensive, borrowed→owned. `into_`: variable, owned→owned (non-Copy). `as_`/`into_` typically decrease abstraction level. `to_` stays at the same level but changes representation. Single-value wrappers use `into_inner()`. Mut ordering: `as_mut_slice` not `as_slice_mut`.

**C-GETTER: Getter names follow Rust convention.**
No `get_` prefix. Field `first` has getter `fn first(&self)` and `fn first_mut(&mut self)`. `get` only when there is a single obvious thing to get (e.g., `Cell::get`). For getters with runtime validation, provide `_unchecked` variants.

**C-ITER: Methods on collections that produce iterators follow `iter`/`iter_mut`/`into_iter`.**
`iter()` → `Iterator<Item = &U>`, `iter_mut()` → `Iterator<Item = &mut U>`, `into_iter()` → `Iterator<Item = U>`. Non-homogeneous types use domain names (`bytes()`, `chars()`).

**C-ITER-TY: Iterator type names match the methods that produce them.**
`into_iter()` returns `IntoIter`, `iter()` returns `Iter`, `keys()` returns `Keys`. Prefixed by owning module: `vec::IntoIter`.

**C-FEATURE: Feature names are free of placeholder words.**
Name features directly: `abc` not `use-abc` or `with-abc`. The optional std dependency is `std` not `use-std`. Features must be additive — never `no-abc`.

**C-WORD-ORDER: Names use a consistent word order.**
Follow stdlib verb-object-error order: `ParseBoolError`, `ParseIntError`, `JoinPathsError`.

## Interoperability

**C-COMMON-TRAITS: Types eagerly implement common traits.**
Due to the orphan rule, implement all applicable traits early: `Copy`, `Clone`, `Eq`, `PartialEq`, `Ord`, `PartialOrd`, `Hash`, `Debug`, `Display`, `Default`. Types implementing `Default` should also have a `new()` constructor with matching behavior.

**C-CONV-TRAITS: Conversions use standard traits `From`, `TryFrom`, `AsRef`, `AsMut`.**
Implement `From`/`TryFrom`. NEVER implement `Into`/`TryInto` directly — they have blanket impls from `From`/`TryFrom`.

**C-COLLECT: Collections implement `FromIterator` and `Extend`.**
Enables `Iterator::collect`, `Iterator::partition`, `Iterator::unzip`.

**C-SERDE: Data structures implement Serde's `Serialize`/`Deserialize`.**
Gate behind a Cargo feature named exactly `"serde"`. Use `#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]`.

**C-SEND-SYNC: Types are `Send` and `Sync` where possible.**
Auto-implemented by compiler. Be vigilant with raw pointers. Test with: `fn assert_send<T: Send>() {} assert_send::<MyType>();`

**C-GOOD-ERR: Error types are meaningful and well-behaved.**
Always implement `std::error::Error`. Must be `Send + Sync`. For trait objects, use `Error + Send + Sync + 'static` (enables `downcast_ref`). NEVER use `()` as an error type. Error messages from `Display`: lowercase, no trailing punctuation. Do NOT implement the deprecated `Error::description()`.

**C-NUM-FMT: Binary number types provide `Hex`/`Octal`/`Binary` formatting.**
Implement `UpperHex`, `LowerHex`, `Octal`, `Binary` for types with bitwise operations.

**C-RW-VALUE: Generic reader/writer functions take `R: Read`/`W: Write` by value.**
Since `&mut R` implements `Read` when `R: Read`, accepting by value lets callers pass `&mut f` when they need to reuse the reader.

## Macros

**C-EVOCATIVE: Input syntax is evocative of the output.**
Mirror existing Rust syntax in macro inputs. If the macro declares a struct, use `struct` keyword.

**C-MACRO-ATTR: Item macros compose well with attributes.**
Support `#[cfg(...)]` on individual items, `#[derive(...)]` on output structs/enums.

**C-ANYWHERE: Item macros work anywhere that items are allowed.**
Must work at module scope AND within function scope.

**C-MACRO-VIS: Item macros support visibility specifiers.**
Accept `pub struct` vs `struct`. Private by default.

**C-MACRO-TY: Type fragments are flexible.**
`$t:ty` must work with: primitives, relative paths, absolute paths, `super::` paths, generics.

## Documentation

**C-CRATE-DOC: Crate-level docs are thorough and include examples.**
The crate root (`lib.rs`) should have comprehensive documentation.

**C-EXAMPLE: All items have a rustdoc example.**
Every public module, trait, struct, enum, function, method, macro, and type definition should have an example showing *why*, not just *how*.

**C-QUESTION-MARK: Examples use `?`, not `try!`, not `unwrap`.**
Use the `fn main() -> Result<(), Box<dyn Error>>` pattern in doc examples.

**C-FAILURE: Function docs include error, panic, and safety conditions.**
`# Errors` section: conditions for returning an error. `# Panics` section: conditions that cause a panic. `# Safety` section: for `unsafe` functions, all invariants the caller must uphold.

**C-LINK: Prose contains hyperlinks to relevant things.**
Link to related types, methods, and traits using rustdoc link syntax.

**C-METADATA: `Cargo.toml` includes all common metadata.**
Required: `authors`, `description`, `license`, `repository`, `keywords`, `categories`.

**C-RELNOTES: Release notes document all significant changes.**
Breaking changes must be clearly identified. Every release should have a corresponding Git tag.

**C-HIDDEN: Rustdoc does not show unhelpful implementation details.**
Use `#[doc(hidden)]` for impls users can never use. Use `pub(crate)` to keep items out of the public API.

## Predictability

**C-SMART-PTR: Smart pointers do not add inherent methods.**
Use associated functions: `Box::into_raw(b)` not `b.into_raw()`. Prevents confusion with methods on the inner type via `Deref`.

**C-CONV-SPECIFIC: Conversions live on the most specific type involved.**
Prefer `to_`/`as_`/`into_` over `from_` (more ergonomic, chainable).

**C-METHOD: Functions with a clear receiver are methods.**
Prefer `impl Foo { pub fn frob(&self, w: Widget) }` over `pub fn frob(foo: &Foo, w: Widget)`. Methods give autoborrowing, discoverability, `self` notation.

**C-NO-OUT: Functions do not take out-parameters.**
Return tuples/structs. Exception: modifying caller-owned buffers (`fn read(&mut self, buf: &mut [u8])`).

**C-OVERLOAD: Operator overloads are unsurprising.**
Only implement `Mul` for multiplication-like operations. Must meet expected mathematical properties.

**C-DEREF: Only smart pointers implement `Deref`/`DerefMut`.**
These traits interact with method resolution. Only for: `Box<T>`, `String`, `Rc<T>`, `Arc<T>`, `Cow<'a, T>`.

**C-CTOR: Constructors are static, inherent methods.**
Primary: `fn new() -> Self`. Secondary: `_with_foo` suffix. Conversion: `from_` prefix. `from_` constructors can be `unsafe` and take extra args (unlike `From` trait). Types with both `Default` and `new()` should have matching behavior.

## Flexibility

**C-INTERMEDIATE: Functions expose intermediate results to avoid duplicate work.**
Return useful intermediate data: `Vec::binary_search` returns index OR insertion point. `String::from_utf8` error exposes byte offset and returns ownership of input bytes.

**C-CALLER-CONTROL: Caller decides where to copy and place data.**
Need ownership: take owned arg. Don't need ownership: take a borrow. Don't borrow+clone. Don't take ownership+drop unnecessarily.

**C-GENERIC: Functions minimize assumptions about parameters by using generics.**
Prefer `fn foo(path: impl AsRef<Path>)` over `fn foo(path: &str)`. Use `IntoIterator` over `&[T]` or `&Vec<T>`. Trade-offs: code size from monomorphization, signature verbosity.

**C-OBJECT: Traits are object-safe if they may be useful as a trait object.**
If meant as object: methods should take/return trait objects, not generics. Use `where Self: Sized` to exclude specific methods from the vtable.

## Type safety

**C-NEWTYPE: Newtypes provide static distinctions.**
`struct Miles(pub f64)` vs `struct Kilometers(pub f64)`. Zero runtime cost. Prevents confusion at compile time.

**C-CUSTOM-TYPE: Arguments convey meaning through types, not `bool` or `Option`.**
`Widget::new(Size::Small, Shape::Round)` not `Widget::new(true, false)`. Easier to extend later.

**C-BITFLAG: Types for a set of flags are `bitflags`, not enums.**
Enums = exactly one choice. Bitflags = presence/absence of multiple flags.

**C-BUILDER: Builders enable construction of complex values.**
Non-consuming builders (preferred): config methods take `&mut self` → `&mut Self`, terminal takes `&self`. Consuming builders: all methods take/return `self`. Non-consuming supports both one-liners and multi-step config.

## Dependability

**C-VALIDATE: Functions validate their arguments.**
In order of preference: (1) static enforcement via types, (2) dynamic enforcement returning `Result`/`Option`, (3) `debug_assert!` for expensive checks, (4) `_unchecked` variants for opt-out.

**C-DTOR-FAIL: Destructors never fail.**
Destructors run during panics; failing causes abort. Provide a separate `close()` method returning `Result`.

**C-DTOR-BLOCK: Destructors that may block have alternatives.**
Provide a separate method for infallible, nonblocking teardown.

## Debuggability

**C-DEBUG: All public types implement `Debug`.**
Exceptions are extremely rare.

**C-DEBUG-NONEMPTY: `Debug` representation is never empty.**
Even empty values produce non-empty output: `""` debugs as `"\"\""`, empty vec as `"[]"`.

## Future proofing

**C-SEALED: Sealed traits protect against downstream implementations.**
Use a private `Sealed` supertrait to prevent external implementations while allowing addition of new methods.

```rust
mod private { pub trait Sealed {} }
pub trait MyTrait: private::Sealed { /* ... */ }
```

**C-STRUCT-PRIVATE: Structs have private fields.**
Public fields pin representation and prevent validation. Use getters/setters.

**C-NEWTYPE-HIDE: Newtypes encapsulate implementation details.**
Wrap complex return types (`Enumerate<Skip<I>>`) in a newtype. Alternative: `impl Trait` (more concise but more limited).

**C-STRUCT-BOUNDS: Data structures do not duplicate derived trait bounds.**
Do NOT put derivable traits as bounds on struct type parameters. `struct Good<T> { ... }` not `struct Bad<T: Clone> { ... }`. Adding a trait bound to a data structure is a breaking change. Never bound on: `Clone`, `PartialEq`, `PartialOrd`, `Debug`, `Display`, `Default`, `Error`, `Serialize`, `Deserialize`.

## Necessities

**C-STABLE: Public dependencies of a stable crate are stable.**
A crate ≥ 1.0.0 cannot have unstable public dependencies.

**C-PERMISSIVE: Crate and its dependencies have a permissive license.**
Recommended: dual license `MIT OR Apache-2.0`, matching the Rust project.
