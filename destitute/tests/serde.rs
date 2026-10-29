#![cfg(feature = "serde")]

use destitute::Destitute;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Destitute, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[destitute(serde, derive(Debug, PartialEq, Default, Clone))]
#[destitute(enum_derive(Debug, PartialEq, Clone))]
struct User {
    #[destitute(mandatory)]
    id: u64,
    name: String,
    age: u8,
    nickname: Option<String>,
}

fn user() -> User {
    User {
        id: 7,
        name: "ana".into(),
        age: 30,
        nickname: None,
    }
}

#[test]
fn enum_deserializes_complete_when_all_fields_present() {
    let v = json!({ "id": 7, "name": "ana", "age": 30 });
    let got: MaybeDestituteUser = serde_json::from_value(v).unwrap();
    assert_eq!(got, MaybeDestituteUser::Complete(user()));
}

#[test]
fn enum_falls_back_to_destitute_when_a_field_is_missing() {
    let v = json!({ "id": 7, "name": "ana" });
    let got: MaybeDestituteUser = serde_json::from_value(v).unwrap();
    assert_eq!(
        got,
        MaybeDestituteUser::Destitute(DestituteUser {
            id: 7,
            name: Some("ana".into()),
            age: None,
            nickname: None,
        })
    );
    assert_eq!(got.try_undestitute().unwrap_err().field, "age");
}

#[test]
fn enum_falls_back_to_destitute_when_a_field_is_null() {
    let v = json!({ "id": 7, "name": null, "age": 30 });
    let got: MaybeDestituteUser = serde_json::from_value(v).unwrap();
    assert!(matches!(got, MaybeDestituteUser::Destitute(_)));
    assert_eq!(got.try_undestitute().unwrap_err().field, "name");
}

#[test]
fn enum_rejects_payload_missing_a_mandatory_field() {
    let v = json!({ "name": "ana", "age": 30 });
    assert!(serde_json::from_value::<MaybeDestituteUser>(v).is_err());
}

#[test]
fn destitute_deserializes_with_only_mandatory_fields() {
    let got: DestituteUser = serde_json::from_value(json!({ "id": 1 })).unwrap();
    assert_eq!(
        got,
        DestituteUser {
            id: 1,
            ..Default::default()
        }
    );
    assert!(serde_json::from_value::<DestituteUser>(json!({})).is_err());
}

#[test]
fn enum_serializes_transparently_and_round_trips() {
    let complete = MaybeDestituteUser::Complete(user());
    let json = serde_json::to_value(&complete).unwrap();
    assert_eq!(json, serde_json::to_value(user()).unwrap());
    assert_eq!(
        serde_json::from_value::<MaybeDestituteUser>(json).unwrap(),
        complete
    );

    let partial = MaybeDestituteUser::Destitute(DestituteUser {
        id: 2,
        ..Default::default()
    });
    let json = serde_json::to_value(&partial).unwrap();
    assert_eq!(
        json,
        json!({ "id": 2, "name": null, "age": null, "nickname": null })
    );
    assert_eq!(
        serde_json::from_value::<MaybeDestituteUser>(json).unwrap(),
        partial
    );
}

#[test]
fn missing_field_error_is_serializable() {
    let err = DestituteUser {
        id: 1,
        ..Default::default()
    }
    .try_undestitute()
    .unwrap_err();
    assert_eq!(
        serde_json::to_value(err).unwrap(),
        json!({ "type_name": "User", "field": "name" })
    );
}
