//! Classify credential-bearing field names and URL components.

pub(super) fn sensitive_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase().replace('-', "_");
    if matches!(name.as_str(), "rsa_public_key" | "ecdsa_public_key" | "token_type_hint") {
        return false;
    }
    if matches!(name.as_str(), "key" | "keys" | "auth")
        || name.ends_with("_key")
        || name.ends_with("_auth")
    {
        return true;
    }
    [
        "password",
        "secret",
        "token",
        "credential",
        "authorization",
        "cookie",
        "api_key",
        "apikey",
        "private_key",
        "privatekey",
        "passphrase",
    ]
    .iter()
    .any(|part| name.contains(part))
}

pub(super) fn sensitive_field(parent: &str, key: &str) -> bool {
    sensitive_name(key) || matches!(parent, "variables" | "credentials" | "keys")
}

pub(super) fn url_has_credentials(string: &str) -> bool {
    url::Url::parse(string).is_ok_and(|url| {
        !url.username().is_empty()
            || url.password().is_some()
            || url.query_pairs().any(|(key, _)| sensitive_name(&key))
    })
}
