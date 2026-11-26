# destitute

A derive macro that gives any struct a *destitute* twin: the same struct, but
with every field made optional. Converting into the twin always succeeds;
converting back fails with the name of the first field that is missing.

```toml
[dependencies]
destitute = "0.1"
```

`no_std`, no runtime dependencies beyond the derive itself (serde is optional).

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

## serde

Enable the `serde` feature and add `serde` to the struct options:

```toml
destitute = { version = "0.1", features = ["serde"] }
```

```rust
#[derive(Destitute, Serialize, Deserialize)]
#[destitute(serde, derive(Default))]
struct User {
    #[destitute(mandatory)]
    id: u64,
    name: String,
    age: u8,
}
```

This derives `Serialize` and `Deserialize` on both generated types, with:

- `#[serde(default)]` on every optional field of `DestituteUser`, so a
  missing key is `None` rather than an error;
- `#[serde(untagged)]` on `MaybeDestituteUser`, `Complete` first. A payload
  becomes `Complete(User)` when every field is there and valid, and
  `Destitute(DestituteUser)` otherwise. Only a missing mandatory field makes
  deserialization fail.

```rust
let full: MaybeDestituteUser = serde_json::from_str(r#"{"id":7,"name":"ana","age":30}"#)?;
assert!(matches!(full, MaybeDestituteUser::Complete(_)));

let partial: MaybeDestituteUser = serde_json::from_str(r#"{"id":7,"name":"ana"}"#)?;
assert!(matches!(partial, MaybeDestituteUser::Destitute(_)));
assert_eq!(partial.try_undestitute().unwrap_err().field, "age");
```

Serialization is transparent: the enum serializes as whichever struct it
holds. Your own struct still needs its own serde derives, and extra serde
attributes go through `attr(serde(..))` / `enum_attr(serde(..))`, or the
field-level `attr(serde(..))` for a single field.

## Field rules

| original field | destitute field | `into_destitute` | `try_undestitute` |
|---|---|---|---|
| `T` | `Option<T>` | `Some(v)` | fails if `None` |
| `#[destitute(mandatory)] T` | `T` | moved | moved |
| `Option<T>` | `Option<T>` | moved | moved, `None` is fine |

`Option<T>` is detected by spelling: `Option<T>`, `core::option::Option<T>` and
`std::option::Option<T>`. A type alias that hides an `Option` is wrapped like
any other type.

## Field options

Inside `#[destitute(...)]` on a field, comma separated:

| option | effect |
|---|---|
| `mandatory` | keep the field as is instead of wrapping it in `Option` |
| `attr(...)` | attributes added verbatim to the field of the destitute struct, e.g. `attr(builder(default))` |

Apart from doc comments, no attribute of the original field is copied: the
twin's field type differs and the twin may not derive the same macros. `attr`
is the opt-in. It is written against the *generated* field, so on a wrapped
field the attribute sees `Option<T>`, not `T`:

```rust
#[derive(Destitute, bon::Builder)]
#[destitute(derive(bon::Builder))]
struct Customer {
    #[destitute(mandatory)]
    email: Email,
    // `HashMap<..>` in both structs, optional setter in both builders.
    #[builder(default)]
    #[destitute(mandatory, attr(builder(default)))]
    store_ids: HashMap<StoreId, CustomerId>,
    // `Option<String>` in the twin, so the attribute must suit an `Option`.
    #[destitute(attr(builder(name = display_name)))]
    name: String,
}
```

## Struct options

All go inside `#[destitute(...)]` on the struct, comma separated, and may be
split across several attributes.

| option | effect |
|---|---|
| `name = "Ident"` | name of the destitute struct (default `Destitute<Struct>`) |
| `derive(A, B)` | derives added to the destitute struct |
| `attr(...)` | one attribute added verbatim to the destitute struct, e.g. `attr(serde(default))` |
| `no_enum` | do not generate the enum |
| `serde` | with the `serde` feature: derive `Serialize`/`Deserialize` on both generated types (see above) |
| `enum_name = "Ident"` | name of the enum (default `MaybeDestitute<Struct>`) |
| `enum_derive(A, B)` | derives added to the enum |
| `enum_attr(...)` | one attribute added verbatim to the enum |

Nothing is derived on the generated types unless you ask for it, so they never
demand bounds your original type does not satisfy. Doc comments on fields are
carried over to the destitute struct; other field attributes only through the
field-level `attr(...)`.

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
cargo test --workspace --all-features
```

The workspace has two crates: `destitute` (the public facade, owns
`MissingField`) and `destitute-derive` (the proc-macro). Commits follow
Conventional Commits and are checked by the hooks.

## License

MIT OR Apache-2.0
