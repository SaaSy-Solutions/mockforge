//! SAML Response XML-DSig verification and claim extraction.
//!
//! This is the only place SAML signatures are verified and the only place
//! assertion claims are read. The two are deliberately one operation: claims
//! are read exclusively from the single `Assertion` node that was proven to be
//! covered by a verified `Reference`, which is the defense against XML
//! Signature Wrapping (XSW).
//!
//! Cryptography and canonicalization are delegated to `bergshamra` (pure Rust,
//! `forbid(unsafe_code)`). Nothing here hand-rolls C14N or digest handling.
//!
//! Security policy enforced on top of the library:
//! - DTDs are forbidden at parse time (no entity tricks).
//! - Only keys from the org's configured certificate are trusted; inline
//!   `KeyInfo` keys are ignored (`trusted_keys_only`).
//! - Every `Signature` in the document must verify; one invalid signature
//!   rejects the whole response.
//! - Each signature has exactly one `Reference`, and its transforms are limited
//!   to enveloped-signature plus (exclusive or inclusive) C14N. No XPath or
//!   XSLT transforms that could narrow what the digest covers.
//! - Reference targets must be the document element, an ancestor, or a sibling
//!   of the `Signature` (`strict_verification`).
//! - The document must contain exactly one `Assertion`, a direct child of the
//!   `Response`, with no `EncryptedAssertion`.
//! - That assertion must be the node a verified `Reference` resolved to, or a
//!   descendant of such a node. Policy flags additionally require the Response
//!   itself and/or the Assertion itself to be a verified Reference target.

use bergshamra::{
    dsig::verify::verify_all_document_with_source,
    keys::loader::load_x509_cert_der,
    xml::uppsala::{Document, NodeId, Parser},
    DsigContext, KeysManager, VerifyResult,
};

const SAML_ASSERTION_NS: &str = "urn:oasis:names:tc:SAML:2.0:assertion";
const SAML_PROTOCOL_NS: &str = "urn:oasis:names:tc:SAML:2.0:protocol";
const DSIG_NS: &str = "http://www.w3.org/2000/09/xmldsig#";

/// Upper bound on a SAML response we are willing to parse and canonicalize.
pub(crate) const MAX_SAML_RESPONSE_BYTES: usize = 1024 * 1024;

const ALLOWED_TRANSFORMS: &[&str] = &[
    "http://www.w3.org/2000/09/xmldsig#enveloped-signature",
    "http://www.w3.org/2001/10/xml-exc-c14n#",
    "http://www.w3.org/TR/2001/REC-xml-c14n-20010315",
    "http://www.w3.org/2006/12/xml-c14n11",
];

/// Which parts of the Response must themselves be signed.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SignaturePolicy {
    pub require_signed_response: bool,
    pub require_signed_assertion: bool,
}

/// One `Attribute` from the verified assertion's `AttributeStatement`.
#[derive(Debug, Clone)]
pub(crate) struct VerifiedAttribute {
    pub name: Option<String>,
    pub friendly_name: Option<String>,
    pub values: Vec<String>,
}

/// Claims read from the single, signature-covered assertion. Raw strings: the
/// caller parses timestamps.
#[derive(Debug, Clone, Default)]
pub(crate) struct VerifiedAssertion {
    pub assertion_id: Option<String>,
    pub issue_instant: Option<String>,
    pub name_id: Option<String>,
    pub session_index: Option<String>,
    pub not_before: Option<String>,
    pub not_on_or_after: Option<String>,
    pub attributes: Vec<VerifiedAttribute>,
}

/// Verify `xml` against the IdP certificate (DER) and return the claims of the
/// one assertion the signature covers. Any failure is fail-closed.
pub(crate) fn verify_and_extract(
    xml: &str,
    cert_der: &[u8],
    policy: SignaturePolicy,
) -> Result<VerifiedAssertion, String> {
    if xml.len() > MAX_SAML_RESPONSE_BYTES {
        return Err("SAML response too large".to_string());
    }

    let doc = Parser::new()
        .with_forbid_dtd(true)
        .parse(xml)
        .map_err(|e| format!("SAML response is not well-formed XML: {e}"))?;

    let response = doc.document_element().ok_or_else(|| "empty SAML document".to_string())?;
    let is_response = doc
        .element(response)
        .is_some_and(|e| e.matches_name_ns(SAML_PROTOCOL_NS, "Response"));
    if !is_response {
        return Err("document element is not a SAML Response".to_string());
    }

    let assertion = single_assertion(&doc, response)?;

    let key = load_x509_cert_der(cert_der).map_err(|e| format!("unusable IdP certificate: {e}"))?;
    let mut keys = KeysManager::new();
    keys.add_key(key);
    // `DsigContext::new` is the hardened constructor: trusted_keys_only,
    // strict (XSW) reference positions, required local digest coverage.
    let ctx = DsigContext::new(keys);

    let results = verify_all_document_with_source(&ctx, &doc, Some(xml))
        .map_err(|e| format!("signature could not be verified: {e}"))?;

    let mut verified: Vec<NodeId> = Vec::new();
    for result in &results {
        match result {
            VerifyResult::Valid {
                signature_node,
                references,
                ..
            } => {
                check_signature_shape(&doc, *signature_node)?;
                if !result.all_reference_digests_verified() {
                    return Err("a Reference digest was not verified".to_string());
                }
                for r in references {
                    match r.resolved_node {
                        Some(node) => verified.push(node),
                        // bergshamra reports no node for whole-document references
                        // (URI="" or #xpointer(/)), the most common real-IdP style for
                        // an enveloped Response signature. The digest covers the whole
                        // document (enveloped transform removes only the Signature
                        // itself), i.e. the Response. Digest verification was checked
                        // above via all_reference_digests_verified.
                        None if r.uri.is_empty() || r.uri == "#xpointer(/)" => {
                            verified.push(response)
                        }
                        None => {}
                    }
                }
            }
            VerifyResult::Invalid { reason } => {
                return Err(format!("invalid signature: {reason}"));
            }
        }
    }

    if policy.require_signed_response && !verified.contains(&response) {
        return Err("SAML Response is not covered by a verified signature".to_string());
    }
    if policy.require_signed_assertion && !verified.contains(&assertion) {
        return Err("SAML Assertion is not covered by a verified signature".to_string());
    }
    // XSW binding: whatever the policy flags, the assertion we read claims
    // from must be a verified Reference target or inside one.
    let covered = verified.contains(&assertion)
        || doc.ancestors(assertion).iter().any(|a| verified.contains(a));
    if !covered {
        return Err("SAML Assertion is not covered by any verified signature".to_string());
    }

    Ok(extract_claims(&doc, assertion))
}

/// Exactly one `Assertion` anywhere in the document, in the SAML namespace and
/// a direct child of the Response; no `EncryptedAssertion`.
fn single_assertion(doc: &Document<'_>, response: NodeId) -> Result<NodeId, String> {
    if !doc.get_elements_by_tag_name("EncryptedAssertion").is_empty() {
        return Err("encrypted assertions are not supported".to_string());
    }
    let all = doc.get_elements_by_tag_name("Assertion");
    if all.len() != 1 {
        return Err(format!("expected exactly one Assertion, found {}", all.len()));
    }
    let assertion = all[0];
    let in_ns = doc
        .element(assertion)
        .is_some_and(|e| e.matches_name_ns(SAML_ASSERTION_NS, "Assertion"));
    if !in_ns {
        return Err("Assertion is not in the SAML assertion namespace".to_string());
    }
    if doc.parent(assertion) != Some(response) {
        return Err("Assertion is not a direct child of the Response".to_string());
    }
    Ok(assertion)
}

/// A SAML signature carries exactly one Reference and only
/// enveloped-signature / C14N transforms.
fn check_signature_shape(doc: &Document<'_>, signature: NodeId) -> Result<(), String> {
    let signed_info = doc
        .child_elements_by_name_ns(signature, DSIG_NS, "SignedInfo")
        .first()
        .copied()
        .ok_or_else(|| "Signature has no SignedInfo".to_string())?;
    let references = doc.child_elements_by_name_ns(signed_info, DSIG_NS, "Reference");
    if references.len() != 1 {
        return Err(format!(
            "Signature must have exactly one Reference, found {}",
            references.len()
        ));
    }
    // bergshamra applies Transforms/Transform by local name, so match by local
    // name regardless of namespace (a foreign-namespace Transform must still be
    // checked against the allowlist).
    for transforms in children_by_local_name(doc, references[0], "Transforms") {
        for transform in children_by_local_name(doc, transforms, "Transform") {
            let alg = doc.get_attribute(transform, "Algorithm").unwrap_or("");
            if !ALLOWED_TRANSFORMS.contains(&alg) {
                return Err(format!("disallowed Reference transform: {alg}"));
            }
        }
    }
    Ok(())
}

fn children_by_local_name(doc: &Document<'_>, parent: NodeId, local: &str) -> Vec<NodeId> {
    doc.children(parent)
        .into_iter()
        .filter(|c| doc.element(*c).is_some_and(|e| *e.name.local_name == *local))
        .collect()
}

fn text_of(doc: &Document<'_>, node: NodeId) -> Option<String> {
    let t = doc.text_content_deep(node);
    let t = t.trim();
    (!t.is_empty()).then(|| t.to_string())
}

fn child(doc: &Document<'_>, parent: NodeId, local: &str) -> Option<NodeId> {
    doc.child_elements_by_name_ns(parent, SAML_ASSERTION_NS, local).first().copied()
}

/// Read claims strictly from the (verified) assertion subtree, using fixed
/// structural paths rather than document-wide searches.
fn extract_claims(doc: &Document<'_>, assertion: NodeId) -> VerifiedAssertion {
    let attr = |node: NodeId, name: &str| doc.get_attribute(node, name).map(str::to_string);

    let mut out = VerifiedAssertion {
        assertion_id: attr(assertion, "ID"),
        issue_instant: attr(assertion, "IssueInstant"),
        ..Default::default()
    };

    out.name_id = child(doc, assertion, "Subject")
        .and_then(|s| child(doc, s, "NameID"))
        .and_then(|n| text_of(doc, n));

    if let Some(conditions) = child(doc, assertion, "Conditions") {
        out.not_before = attr(conditions, "NotBefore");
        out.not_on_or_after = attr(conditions, "NotOnOrAfter");
    }

    out.session_index = doc
        .child_elements_by_name_ns(assertion, SAML_ASSERTION_NS, "AuthnStatement")
        .into_iter()
        .find_map(|s| attr(s, "SessionIndex"));

    for statement in
        doc.child_elements_by_name_ns(assertion, SAML_ASSERTION_NS, "AttributeStatement")
    {
        for a in doc.child_elements_by_name_ns(statement, SAML_ASSERTION_NS, "Attribute") {
            let values = doc
                .child_elements_by_name_ns(a, SAML_ASSERTION_NS, "AttributeValue")
                .into_iter()
                .filter_map(|v| text_of(doc, v))
                .collect();
            out.attributes.push(VerifiedAttribute {
                name: attr(a, "Name"),
                friendly_name: attr(a, "FriendlyName"),
                values,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests;
