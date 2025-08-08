use openmodel::common::Data;

#[test]
fn test_data_creation() {
    let data = Data::with_name("MyObject");
    
    assert!(data.guid().is_nil() == false);
    assert_eq!(data.name(), "MyObject");
}

#[test]
fn test_data_creation_with_long_name() {
    // With String implementation, names can be any length
    let data = Data::with_name("ThisNameIsWayTooLongForTheFixedSizeArrayButNowItsOkayBecauseWeUseString");
    assert_eq!(data.name(), "ThisNameIsWayTooLongForTheFixedSizeArrayButNowItsOkayBecauseWeUseString");
}