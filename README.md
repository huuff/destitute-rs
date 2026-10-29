# destitute

Derive an all-optional twin of any struct, and promote it back fallibly.

```rust
use destitute::Destitute;

#[derive(Destitute)]
#[destitute(derive(Debug, Default))]
struct User {
    #[destitute(mandatory)]
    id: u64,
    name: String,
    age: u8,
}

// Generated: DestituteUser { id: u64, name: Option<String>, age: Option<u8> }
let partial = DestituteUser { id: 7, name: Some("ana".into()), ..Default::default() };
let err = partial.try_undestitute().unwrap_err();
assert_eq!(err.to_string(), "missing field `age` while promoting into `User`");

let full = User { id: 7, name: "ana".into(), age: 30 };
let partial = full.into_destitute(); // every non-mandatory field wrapped in Some

// Generated too: enum MaybeUser { Complete(User), Destitute(DestituteUser) }
```

| struct option | effect |
|---|---|
| `name = "Ident"` | name of the destitute struct (default `Destitute<Struct>`) |
| `derive(..)` / `attr(..)` | derives / attributes added to the destitute struct |
| `no_enum` | skip the either-or enum |
| `enum_name = "Ident"` | name of the enum (default `Maybe<Struct>`) |
| `enum_derive(..)` / `enum_attr(..)` | derives / attributes added to the enum |

Field option: `#[destitute(mandatory)]` keeps a field non-optional.

`no_std`, no runtime dependencies. See the crate docs for details.
