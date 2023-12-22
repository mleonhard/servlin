use servlin::internal::SameSite;
use servlin::{AsciiString, Cookie};
use std::time::{Duration, SystemTime};

fn value1() -> AsciiString {
    "value1".try_into().unwrap()
}

#[test]
#[should_panic(expected = "Cookie::new called with empty `name`")]
fn empty_name_should_panic() {
    Cookie::new("", value1()).to_string();
}

#[test]
fn empty_value() {
    assert_eq!(
        Cookie::new("name1", AsciiString::new()).to_string(),
        "name1=; HttpOnly; Max-Age=2592000; SameSite=Strict; Secure",
    );
}

#[test]
fn new() {
    assert_eq!(
        Cookie::new("name1", value1()).to_string(),
        "name1=value1; HttpOnly; Max-Age=2592000; SameSite=Strict; Secure",
    );
}

#[test]
fn with_domain() {
    assert_eq!(
        Cookie::new("name1", value1())
            .with_domain("example.com")
            .to_string(),
        "name1=value1; Domain=example.com; HttpOnly; Max-Age=2592000; SameSite=Strict; Secure",
    );
}

#[allow(clippy::unreadable_literal)]
#[test]
fn with_expires() {
    assert_eq!(
        Cookie::new("name1", value1())
            .with_expires(SystemTime::UNIX_EPOCH)
            .to_string(),
        "name1=value1; HttpOnly; Max-Age=2592000; SameSite=Strict; Secure",
    );
    assert_eq!(
        Cookie::new("name1", value1())
            .with_expires(SystemTime::UNIX_EPOCH + Duration::from_secs(1648690632))
            .to_string(),
        "name1=value1; Expires=2022-03-31T01:37:12Z; HttpOnly; Max-Age=2592000; SameSite=Strict; Secure",
    );
}

#[test]
fn with_http_only_true() {
    assert_eq!(
        Cookie::new("name1", value1())
            .with_http_only(false)
            .to_string(),
        "name1=value1; Max-Age=2592000; SameSite=Strict; Secure",
    );
    assert_eq!(
        Cookie::new("name1", value1())
            .with_http_only(true)
            .to_string(),
        "name1=value1; HttpOnly; Max-Age=2592000; SameSite=Strict; Secure",
    );
}

#[test]
fn with_max_age() {
    assert_eq!(
        Cookie::new("name1", value1())
            .with_max_age(Duration::ZERO)
            .to_string(),
        "name1=value1; HttpOnly; SameSite=Strict; Secure",
    );
    assert_eq!(
        Cookie::new("name1", value1())
            .with_max_age(Duration::from_secs(123))
            .to_string(),
        "name1=value1; HttpOnly; Max-Age=123; SameSite=Strict; Secure",
    );
}

#[test]
fn with_path() {
    assert_eq!(
        Cookie::new("name1", value1()).with_path("").to_string(),
        "name1=value1; HttpOnly; Max-Age=2592000; SameSite=Strict; Secure",
    );
    assert_eq!(
        Cookie::new("name1", value1())
            .with_path("/path1")
            .to_string(),
        "name1=value1; HttpOnly; Max-Age=2592000; Path=/path1; SameSite=Strict; Secure",
    );
}

#[test]
fn with_same_site() {
    assert_eq!(
        Cookie::new("name1", value1())
            .with_same_site(SameSite::Strict)
            .to_string(),
        "name1=value1; HttpOnly; Max-Age=2592000; SameSite=Strict; Secure",
    );
    assert_eq!(
        Cookie::new("name1", value1())
            .with_same_site(SameSite::Lax)
            .to_string(),
        "name1=value1; HttpOnly; Max-Age=2592000; SameSite=Lax; Secure",
    );
    assert_eq!(
        Cookie::new("name1", value1())
            .with_same_site(SameSite::None)
            .to_string(),
        "name1=value1; HttpOnly; Max-Age=2592000; SameSite=None; Secure",
    );
}

#[test]
fn with_secure() {
    assert_eq!(
        Cookie::new("name1", value1())
            .with_secure(false)
            .to_string(),
        "name1=value1; HttpOnly; Max-Age=2592000; SameSite=Strict",
    );
    assert_eq!(
        Cookie::new("name1", value1()).with_secure(true).to_string(),
        "name1=value1; HttpOnly; Max-Age=2592000; SameSite=Strict; Secure",
    );
}
