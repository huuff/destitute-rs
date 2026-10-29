//! Derive an all-optional twin of a struct, and convert both ways.
//!
//! ```
//! use destitute::Destitute;
//!
//! #[derive(Destitute, Debug, PartialEq)]
//! #[destitute(derive(Debug, Default, PartialEq))]
//! struct User {
//!     #[destitute(mandatory)]
//!     id: u64,
//!     name: String,
//!     age: u8,
//! }
//!
//! // `DestituteUser { id: u64, name: Option<String>, age: Option<u8> }`
//! let partial = DestituteUser { id: 7, name: Some("ana".into()), age: None };
//!
//! let err = partial.try_undestitute().unwrap_err();
//! assert_eq!(err.field, "age");
//! assert_eq!(err.to_string(), "missing field `age` while promoting into `User`");
//!
//! let full = User { id: 7, name: "ana".into(), age: 30 };
//! let back = full.into_destitute().try_undestitute().unwrap();
//! assert_eq!(back, User { id: 7, name: "ana".into(), age: 30 });
//!
//! // `MaybeUser` holds either form; it is generated unless `#[destitute(no_enum)]`.
//! let either: MaybeUser = DestituteUser { id: 1, ..Default::default() }.into();
//! assert!(either.try_undestitute().is_err());
//! ```
//!
//! # Attributes
//!
//! On the struct, `#[destitute(...)]` accepts, comma separated:
//!
//! | option | effect |
//! |---|---|
//! | `name = "Ident"` | name of the destitute struct (default `Destitute<Struct>`) |
//! | `derive(A, B)` | derives added to the destitute struct |
//! | `attr(...)` | an arbitrary attribute added to the destitute struct, e.g. `attr(serde(default))` |
//! | `no_enum` | skip the either-or enum |
//! | `enum_name = "Ident"` | name of the enum (default `Maybe<Struct>`) |
//! | `enum_derive(A, B)` | derives added to the enum |
//! | `enum_attr(...)` | an arbitrary attribute added to the enum |
//!
//! On a field, `#[destitute(mandatory)]` keeps the field as is instead of
//! wrapping it in `Option`.
//!
//! # Generated items
//!
//! For `struct S`, with destitute name `D` and enum name `E`:
//!
//! - `struct D`, same visibility, generics and field visibility as `S`;
//!   field doc comments are carried over.
//! - `S::into_destitute(self) -> D` and `impl From<S> for D`.
//! - `D::try_undestitute(self) -> Result<S, MissingField>` and
//!   `impl TryFrom<D> for S`. The first `None` field, in declaration order,
//!   is reported.
//! - `enum E { Complete(S), Destitute(D) }` with `into_destitute`,
//!   `try_undestitute`, `From<S>` and `From<D>`.
//!
//! Fields that are already `Option<T>` are wrapped again (`Option<Option<T>>`);
//! mark them `mandatory` if that is not what you want.
//!
//! The crate is `no_std`.
#![no_std]

use core::fmt;

pub use destitute_derive::Destitute;

/// A `None` field blocked promotion of a destitute value into its complete form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MissingField {
    /// Name of the type that could not be built.
    pub type_name: &'static str,
    /// Name of the first field found to be `None`.
    pub field: &'static str,
}

impl fmt::Display for MissingField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "missing field `{}` while promoting into `{}`",
            self.field, self.type_name
        )
    }
}

impl core::error::Error for MissingField {}
