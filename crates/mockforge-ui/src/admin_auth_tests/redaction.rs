#[test]
fn log_headers_and_proxy_urls_are_redacted() {
    use crate::handlers::{redact_sensitive_headers, redact_url_userinfo};
    let headers = [
        ("Authorization", "Bearer abc"),
        ("cookie", "s=1"),
        ("X-Api-Key", "k"),
        ("x-refresh-token", "t"),
        ("X-MockForge-Management-Token", "m"),
        ("content-type", "application/json"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    let redacted = redact_sensitive_headers(headers);
    for name in [
        "Authorization",
        "cookie",
        "X-Api-Key",
        "x-refresh-token",
        "X-MockForge-Management-Token",
    ] {
        assert_eq!(redacted[name], "[REDACTED]", "{name}");
    }
    assert_eq!(redacted["content-type"], "application/json");

    assert_eq!(redact_url_userinfo("https://u:p@up.example/a@b"), "https://***@up.example/a@b");
    assert_eq!(redact_url_userinfo("https://up.example/a@b"), "https://up.example/a@b");
    assert_eq!(redact_url_userinfo("http://tok@h:1?q=@"), "http://***@h:1?q=@");
    assert_eq!(redact_url_userinfo("not a url"), "not a url");
}
