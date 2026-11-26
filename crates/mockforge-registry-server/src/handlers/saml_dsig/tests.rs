use super::*;

const IDP_CERT: &str = include_str!("../../../tests/fixtures/saml/idp_cert.pem");
const ATTACKER_CERT: &str = include_str!("../../../tests/fixtures/saml/attacker_cert.pem");
const ASSERTION_SIGNED: &str =
    include_str!("../../../tests/fixtures/saml/response_assertion_signed.xml");
const RESPONSE_SIGNED: &str =
    include_str!("../../../tests/fixtures/saml/response_response_signed.xml");
const DSIG_PREFIX: &str =
    include_str!("../../../tests/fixtures/saml/response_assertion_signed_dsigprefix.xml");
const SAML2_PREFIX: &str =
    include_str!("../../../tests/fixtures/saml/response_assertion_signed_saml2prefix.xml");
const ATTACKER_SIGNED: &str =
    include_str!("../../../tests/fixtures/saml/response_attacker_signed.xml");
const ISSUER_ONLY_SIGNED: &str =
    include_str!("../../../tests/fixtures/saml/response_issuer_signed_only.xml");
const XPATH_TRANSFORM: &str =
    include_str!("../../../tests/fixtures/saml/response_assertion_xpath_transform.xml");
const EMPTY_URI_SIGNED: &str =
    include_str!("../../../tests/fixtures/saml/response_response_signed_empty_uri.xml");
const XPATH_FOREIGN_NS: &str =
    include_str!("../../../tests/fixtures/saml/response_assertion_xpath_foreign_ns.xml");
const UNSIGNED: &str = include_str!("../../../tests/fixtures/saml/response_unsigned.xml");

const ASSERTION_ONLY: SignaturePolicy = SignaturePolicy {
    require_signed_response: false,
    require_signed_assertion: true,
};
const RESPONSE_ONLY: SignaturePolicy = SignaturePolicy {
    require_signed_response: true,
    require_signed_assertion: false,
};
const NO_FLAGS: SignaturePolicy = SignaturePolicy {
    require_signed_response: false,
    require_signed_assertion: false,
};
const BOTH: SignaturePolicy = SignaturePolicy {
    require_signed_response: true,
    require_signed_assertion: true,
};

fn der(pem: &str) -> Vec<u8> {
    rustls_pemfile::certs(&mut pem.as_bytes()).next().unwrap().unwrap().to_vec()
}

fn verify(xml: &str, policy: SignaturePolicy) -> Result<VerifiedAssertion, String> {
    verify_and_extract(xml, &der(IDP_CERT), policy)
}

/// Extract the `<saml:Assertion ...>...</saml:Assertion>` text (signed, with prefix `p`).
fn assertion_text<'a>(xml: &'a str, p: &str) -> &'a str {
    let start = xml.find(&format!("<{p}:Assertion")).unwrap();
    let end_tag = format!("</{p}:Assertion>");
    let end = xml.find(&end_tag).unwrap() + end_tag.len();
    &xml[start..end]
}

// ---- positive: real signed responses from an independent signer are accepted

#[test]
fn accepts_assertion_signed_response_and_reads_claims() {
    let v = verify(ASSERTION_SIGNED, ASSERTION_ONLY).expect("must accept");
    assert_eq!(v.name_id.as_deref(), Some("alice@acme.example"));
    assert_eq!(v.assertion_id.as_deref(), Some("_assert1"));
    assert_eq!(v.session_index.as_deref(), Some("_sess1"));
    assert_eq!(v.not_on_or_after.as_deref(), Some("2026-01-01T00:05:00Z"));
    assert!(v
        .attributes
        .iter()
        .any(|a| a.name.as_deref() == Some("givenName") && a.values == vec!["Alice".to_string()]));
}

#[test]
fn accepts_response_signed_response() {
    let v = verify(RESPONSE_SIGNED, RESPONSE_ONLY).expect("must accept");
    assert_eq!(v.name_id.as_deref(), Some("alice@acme.example"));
}

#[test]
fn accepts_prefix_namespaced_signatures_and_saml_elements() {
    assert!(verify(DSIG_PREFIX, ASSERTION_ONLY).is_ok(), "dsig: prefix");
    let v = verify(SAML2_PREFIX, ASSERTION_ONLY).expect("saml2: prefix");
    assert_eq!(v.name_id.as_deref(), Some("alice@acme.example"));
}

// ---- negatives

#[test]
fn rejects_unsigned() {
    for p in [ASSERTION_ONLY, RESPONSE_ONLY, BOTH] {
        assert!(verify(UNSIGNED, p).is_err());
    }
}

#[test]
fn rejects_signature_by_untrusted_key() {
    // Valid signature, embedded KeyInfo cert is the attacker's. Must not be
    // accepted against the org's configured cert.
    assert!(verify(ATTACKER_SIGNED, ASSERTION_ONLY).is_err());
    // And the attacker's own cert is only trusted if the org configured it.
    assert!(verify_and_extract(ATTACKER_SIGNED, &der(ATTACKER_CERT), ASSERTION_ONLY).is_ok());
}

#[test]
fn rejects_tampered_claims() {
    let forged = ASSERTION_SIGNED.replace("alice@acme.example", "admin@victim.example");
    assert_ne!(forged, ASSERTION_SIGNED);
    assert!(verify(&forged, ASSERTION_ONLY).is_err());
    let forged = RESPONSE_SIGNED.replace("alice@acme.example", "admin@victim.example");
    assert!(verify(&forged, RESPONSE_ONLY).is_err());
}

#[test]
fn rejects_stripped_signature() {
    let start = ASSERTION_SIGNED.find("<ds:Signature").unwrap();
    let end = ASSERTION_SIGNED.find("</ds:Signature>").unwrap() + "</ds:Signature>".len();
    let stripped = format!("{}{}", &ASSERTION_SIGNED[..start], &ASSERTION_SIGNED[end..]);
    assert!(verify(&stripped, ASSERTION_ONLY).is_err());
}

#[test]
fn rejects_wrong_policy_coverage() {
    // Only the assertion is signed but the org requires a signed Response.
    assert!(verify(ASSERTION_SIGNED, RESPONSE_ONLY).is_err());
    assert!(verify(ASSERTION_SIGNED, BOTH).is_err());
    // Only the Response is signed but the org requires a signed Assertion.
    assert!(verify(RESPONSE_SIGNED, ASSERTION_ONLY).is_err());
}

#[test]
fn rejects_wrapped_assertion_evil_sibling_first() {
    // XSW: attacker prepends an unsigned evil assertion; the signed one stays.
    let signed = assertion_text(ASSERTION_SIGNED, "saml");
    let evil = r#"<saml:Assertion ID="_evil" Version="2.0" IssueInstant="2026-01-01T00:00:00Z"><saml:Subject><saml:NameID>admin@victim.example</saml:NameID></saml:Subject></saml:Assertion>"#;
    let wrapped = ASSERTION_SIGNED.replace(signed, &format!("{evil}{signed}"));
    assert_ne!(wrapped, ASSERTION_SIGNED);
    assert!(verify(&wrapped, ASSERTION_ONLY).is_err());
    assert!(verify(&wrapped, BOTH).is_err());
}

#[test]
fn rejects_wrapped_assertion_signed_one_hidden_in_extensions() {
    // XSW: evil assertion is the direct child; the validly signed one is
    // moved inside an Extensions wrapper so signature still verifies.
    let signed = assertion_text(ASSERTION_SIGNED, "saml");
    let evil = r#"<saml:Assertion ID="_evil" Version="2.0" IssueInstant="2026-01-01T00:00:00Z"><saml:Subject><saml:NameID>admin@victim.example</saml:NameID></saml:Subject></saml:Assertion>"#;
    let wrapped = ASSERTION_SIGNED
        .replace(signed, &format!("{evil}<samlp:Extensions>{signed}</samlp:Extensions>"));
    assert!(verify(&wrapped, ASSERTION_ONLY).is_err());
}

#[test]
fn rejects_signed_assertion_nested_not_direct_child() {
    // Single assertion, validly signed, but moved under Extensions: not a
    // direct child of Response.
    let signed = assertion_text(ASSERTION_SIGNED, "saml");
    let wrapped =
        ASSERTION_SIGNED.replace(signed, &format!("<samlp:Extensions>{signed}</samlp:Extensions>"));
    assert!(verify(&wrapped, ASSERTION_ONLY).is_err());
}

#[test]
fn rejects_unsigned_assertion_added_to_response_signed_doc() {
    let evil = r#"<saml:Assertion ID="_evil" Version="2.0" IssueInstant="2026-01-01T00:00:00Z"><saml:Subject><saml:NameID>admin@victim.example</saml:NameID></saml:Subject></saml:Assertion>"#;
    let wrapped = RESPONSE_SIGNED.replace("</samlp:Response>", &format!("{evil}</samlp:Response>"));
    assert!(verify(&wrapped, RESPONSE_ONLY).is_err());
}

#[test]
fn rejects_doctype_and_non_response_root_and_oversize() {
    let dtd = format!("<!DOCTYPE x [<!ENTITY a \"b\">]>{ASSERTION_SIGNED}");
    assert!(verify(&dtd, ASSERTION_ONLY).is_err());
    assert!(verify("<Response/>", ASSERTION_ONLY).is_err());
    let big = " ".repeat(MAX_SAML_RESPONSE_BYTES + 1);
    assert!(verify(&big, ASSERTION_ONLY).is_err());
}

#[test]
fn rejects_encrypted_assertion() {
    let wrapped = ASSERTION_SIGNED
        .replace("</samlp:Response>", "<saml:EncryptedAssertion/></samlp:Response>");
    assert!(verify(&wrapped, ASSERTION_ONLY).is_err());
}

#[test]
fn rejects_garbage_cert() {
    assert!(verify_and_extract(ASSERTION_SIGNED, b"not a cert", ASSERTION_ONLY).is_err());
}

#[test]
fn rejects_valid_signature_that_does_not_cover_the_assertion() {
    // The Issuer is validly signed by the IdP key, but the Assertion is not
    // inside any verified Reference. Binding must hold independent of flags.
    assert!(verify(ISSUER_ONLY_SIGNED, NO_FLAGS).is_err());
    for p in [ASSERTION_ONLY, RESPONSE_ONLY, BOTH] {
        assert!(verify(ISSUER_ONLY_SIGNED, p).is_err());
    }
}

#[test]
fn rejects_second_unsigned_assertion_appended_after_signed_one() {
    let signed = assertion_text(ASSERTION_SIGNED, "saml");
    let evil = r#"<saml:Assertion ID="_evil" Version="2.0" IssueInstant="2026-01-01T00:00:00Z"><saml:Subject><saml:NameID>admin@victim.example</saml:NameID></saml:Subject></saml:Assertion>"#;
    let wrapped = ASSERTION_SIGNED.replace(signed, &format!("{signed}{evil}"));
    assert_ne!(wrapped, ASSERTION_SIGNED);
    assert!(verify(&wrapped, ASSERTION_ONLY).is_err());
}

#[test]
fn rejects_response_when_any_extra_signature_is_invalid() {
    // A valid assertion signature plus a second Signature (copy of the first,
    // placed under the Response) whose digest cannot match. One bad signature
    // must reject the whole response.
    let start = ASSERTION_SIGNED.find("<ds:Signature").unwrap();
    let end = ASSERTION_SIGNED.find("</ds:Signature>").unwrap() + "</ds:Signature>".len();
    let sig = &ASSERTION_SIGNED[start..end];
    let doc = ASSERTION_SIGNED.replace("<saml:Assertion ", &format!("{sig}<saml:Assertion "));
    assert_ne!(doc, ASSERTION_SIGNED);
    assert!(verify(&doc, ASSERTION_ONLY).is_err());
}

#[test]
fn rejects_valid_signature_with_xpath_transform() {
    // Cryptographically valid, but XPath transforms can narrow what the digest
    // covers, so they are outside the SAML profile we accept.
    let err = verify(XPATH_TRANSFORM, ASSERTION_ONLY).unwrap_err();
    assert!(err.contains("transform"), "unexpected error: {err}");
}

#[test]
fn accepts_response_signed_with_empty_uri_whole_document_reference() {
    // URI="" (enveloped signature over the whole document) is the most common
    // real-IdP style; bergshamra reports no resolved node for it.
    assert!(EMPTY_URI_SIGNED.contains(r#"Reference URI="""#));
    let v = verify(EMPTY_URI_SIGNED, RESPONSE_ONLY).expect("must accept URI=\"\"");
    assert_eq!(v.name_id.as_deref(), Some("alice@acme.example"));
    // It covers the Response, not the Assertion node itself.
    assert!(verify(EMPTY_URI_SIGNED, BOTH).is_err());
    // Tampering is still caught by the whole-document digest.
    let forged = EMPTY_URI_SIGNED.replace("alice@acme.example", "admin@victim.example");
    assert!(verify(&forged, RESPONSE_ONLY).is_err());
}

#[test]
fn rejects_xpath_transform_in_foreign_namespace() {
    // The Transform element sits in a non-dsig namespace but bergshamra applies
    // it by local name; the allowlist must see it too.
    let err = verify(XPATH_FOREIGN_NS, ASSERTION_ONLY).unwrap_err();
    assert!(err.contains("transform"), "unexpected error: {err}");
}
