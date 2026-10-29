use destitute::{Destitute, MissingField};

#[derive(Destitute, Debug, Clone, PartialEq)]
#[destitute(derive(Debug, Clone, Default, PartialEq))]
#[destitute(enum_derive(Debug, Clone, PartialEq))]
pub struct User {
    /// Always known.
    #[destitute(mandatory)]
    pub id: u64,
    pub name: String,
    pub age: u8,
}

#[test]
fn round_trip() {
    let user = User {
        id: 1,
        name: "ana".into(),
        age: 30,
    };
    let partial = user.clone().into_destitute();
    assert_eq!(
        partial,
        DestituteUser {
            id: 1,
            name: Some("ana".into()),
            age: Some(30)
        }
    );
    assert_eq!(partial.try_undestitute(), Ok(user));
}

#[test]
fn reports_first_missing_field_in_declaration_order() {
    let partial = DestituteUser {
        id: 1,
        name: None,
        age: None,
    };
    assert_eq!(
        partial.try_undestitute(),
        Err(MissingField {
            type_name: "User",
            field: "name"
        })
    );
    let partial = DestituteUser {
        id: 1,
        name: Some("x".into()),
        age: None,
    };
    let err = partial.try_undestitute().unwrap_err();
    assert_eq!(err.field, "age");
    assert_eq!(
        err.to_string(),
        "missing field `age` while promoting into `User`"
    );
}

#[test]
fn destitute_is_mutable_and_defaultable() {
    let mut partial = DestituteUser {
        id: 9,
        ..Default::default()
    };
    partial.name = Some("ana".into());
    partial.age.get_or_insert(18);
    assert!(partial.try_undestitute().is_ok());
}

#[test]
fn from_and_try_from_impls() {
    let user = User {
        id: 2,
        name: "bo".into(),
        age: 5,
    };
    let partial: DestituteUser = user.clone().into();
    let back: Result<User, MissingField> = partial.try_into();
    assert_eq!(back, Ok(user));
}

#[test]
fn either_enum() {
    let user = User {
        id: 3,
        name: "cy".into(),
        age: 9,
    };
    let a: MaybeUser = user.clone().into();
    let b: MaybeUser = user.clone().into_destitute().into();
    assert_eq!(a, MaybeUser::Complete(user.clone()));
    assert!(matches!(b, MaybeUser::Destitute(_)));
    assert_eq!(a.clone().into_destitute(), b.clone().into_destitute());
    assert_eq!(a.try_undestitute(), Ok(user.clone()));
    assert_eq!(b.try_undestitute(), Ok(user));
    let missing: MaybeUser = DestituteUser {
        id: 3,
        ..Default::default()
    }
    .into();
    assert_eq!(missing.try_undestitute().unwrap_err().field, "name");
}

#[derive(Destitute)]
#[destitute(name = "Partial", no_enum, attr(allow(dead_code)))]
struct Renamed<T: Clone>
where
    T: Default,
{
    #[destitute(mandatory)]
    key: T,
    value: Option<T>,
}

#[test]
fn custom_name_generics_and_double_option() {
    let p = Partial::<u8> {
        key: 1,
        value: Some(None),
    };
    let r = p.try_undestitute().unwrap();
    assert_eq!((r.key, r.value), (1, None));
}

#[derive(Destitute)]
#[destitute(enum_name = "AnyPoint")]
struct Point {
    x: i32,
    y: i32,
}

#[test]
fn custom_enum_name() {
    let p: AnyPoint = Point { x: 1, y: 2 }.into();
    assert!(matches!(p, AnyPoint::Complete(_)));
    assert_eq!(p.try_undestitute().map(|p| (p.x, p.y)), Ok((1, 2)));
}
