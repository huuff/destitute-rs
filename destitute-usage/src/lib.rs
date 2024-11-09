use destitute_macro::Destitute;

#[derive(Destitute)]
struct Example {
    #[destitute]
    field1: u8,
    field2: u8,
}
