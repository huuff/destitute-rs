# destitute

A derive macro that gives any struct a *destitute* twin: the same struct, but
with every field made optional. Converting into the twin always succeeds;
converting back fails with the name of the first field that is missing.

```toml
[dependencies]
destitute = "0.1"
```

`no_std`, no runtime dependencies beyond the derive itself.

## The problem

You have a "complete" type that the rest of your program relies on:

```rust
struct User {
    id: u64,
    name: String,
    age: u8,
}
```

Somewhere at the edge, data arrives incomplete: a form that is half filled in,
a PATCH request body, a config file with defaults still to be merged, a row
being assembled across several queries. You want a type for *that* state too,
without hand-writing a second struct and keeping the two in sync forever.

## What the derive gives you

```rust
use destitute::Destitute;

#[derive(Destitute, Debug)]
#[destitute(derive(Debug, Default))]
struct User {
    #[destitute(mandatory)]
    id: u64,
    name: String,
    age: u8,
    nickname: Option<String>,
}
```

Expands to, in prose:

**1. The destitute struct.** Same visibility, generics and field visibility,
named `Destitute<Struct>` by default:

```rust
#[derive(Debug, Default)]           // from #[destitute(derive(..))]
struct DestituteUser {
    id: u64,                        // mandatory: stays as is
    name: Option<String>,           // wrapped
    age: Option<u8>,                // wrapped
    nickname: Option<String>,       // already Option: stays flat
}
```

Fields are plain and mutable, so you fill it in however you like: struct
literal with `..Default::default()`, field assignment, `get_or_insert`, serde,
a builder, whatever.

**2. Going down is infallible.** Every wrapped field becomes `Some`:

```rust
impl User {
    fn into_destitute(self) -> DestituteUser { .. }
}
impl From<User> for DestituteUser { .. }
```

**3. Coming back up is fallible.** The first wrapped field that is `None`, in
declaration order, aborts the promotion and is named in the error:

```rust
impl DestituteUser {
    fn try_undestitute(self) -> Result<User, destitute::MissingField> { .. }
}
impl TryFrom<DestituteUser> for User { type Error = destitute::MissingField; .. }
```

```rust
let partial = DestituteUser { id: 7, name: Some("ana".into()), ..Default::default() };
let err = partial.try_undestitute().unwrap_err();
assert_eq!(err.field, "age");
assert_eq!(err.type_name, "User");
assert_eq!(err.to_string(), "missing field `age` while promoting into `User`");
```

`MissingField` is a plain `Copy` struct with two `&'static str` fields and
implements `Display` and `Error`, so it slots into `?`, `anyhow`, `thiserror`
`#[from]`, and friends.

**4. An either-or enum**, for APIs that accept both shapes. Named
`MaybeDestitute<Struct>` by default; opt out with `no_enum`:

```rust
enum MaybeDestituteUser {
    Complete(User),
    Destitute(DestituteUser),
}
impl MaybeDestituteUser {
    fn into_destitute(self) -> DestituteUser { .. }      // flattens Complete
    fn try_undestitute(self) -> Result<User, MissingField> { .. } // Complete is Ok
}
impl From<User> for MaybeDestituteUser { .. }
impl From<DestituteUser> for MaybeDestituteUser { .. }
```

Pair it with `#[destitute(enum_attr(serde(untagged)))]` and you can
deserialize either form from the same payload, then call `try_undestitute`
once when you actually need the complete type.

## Field rules

| original field | destitute field | `into_destitute` | `try_undestitute` |
|---|---|---|---|
| `T` | `Option<T>` | `Some(v)` | fails if `None` |
| `#[destitute(mandatory)] T` | `T` | moved | moved |
| `Option<T>` | `Option<T>` | moved | moved, `None` is fine |

`Option<T>` is detected by spelling: `Option<T>`, `core::option::Option<T>` and
`std::option::Option<T>`. A type alias that hides an `Option` is wrapped like
any other type.

## Struct options

All go inside `#[destitute(...)]` on the struct, comma separated, and may be
split across several attributes.

| option | effect |
|---|---|
| `name = "Ident"` | name of the destitute struct (default `Destitute<Struct>`) |
| `derive(A, B)` | derives added to the destitute struct |
| `attr(...)` | one attribute added verbatim to the destitute struct, e.g. `attr(serde(default))` |
| `no_enum` | do not generate the enum |
| `enum_name = "Ident"` | name of the enum (default `MaybeDestitute<Struct>`) |
| `enum_derive(A, B)` | derives added to the enum |
| `enum_attr(...)` | one attribute added verbatim to the enum |

Nothing is derived on the generated types unless you ask for it, so they never
demand bounds your original type does not satisfy. Doc comments on fields are
carried over to the destitute struct.

## Limits

- Structs with named fields only. Tuple and unit structs, and enums, are a
  compile error.
- The generated code refers to `::destitute::MissingField`, so the `destitute`
  crate must be a direct dependency under that name.
- Generics and `where` clauses are copied as written; the derive adds no
  bounds of its own.

## Development

```sh
devenv shell          # or `direnv allow`; installs git hooks
cargo test --workspace
```

The workspace has two crates: `destitute` (the public facade, owns
`MissingField`) and `destitute-derive` (the proc-macro). Commits follow
Conventional Commits and are checked by the hooks.

## License

MIT OR Apache-2.0
