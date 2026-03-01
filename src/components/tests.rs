use crate::components::baker::layout::is_remote_newer;

#[test]
fn test_is_remote_newer() {
    assert!(is_remote_newer("1.0.0", "1.0.1").unwrap());
    assert!(is_remote_newer("0.0.0", "0.0.1").unwrap());
    assert!(is_remote_newer("1.0.0-alpha.1", "1.0.0-alpha.2").unwrap());
    assert!(is_remote_newer("1.0.0-alpha.1", "1.0.0-beta.2").unwrap());
    assert!(is_remote_newer("1.0.0-alpha.1", "1.0.0").unwrap());
    assert!(!is_remote_newer("1.0.2", "1.0.2").unwrap());
    assert!(!is_remote_newer("1.0.2", "1.0.1").unwrap());
}
