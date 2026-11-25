"""Regenerates the SAML test fixtures with an independent signer (signxml/lxml/libxml2).

Test keys only. Private keys are NOT committed. To regenerate everything:
  openssl req -x509 -newkey rsa:2048 -nodes -keyout idp_key.pem -out idp_cert.pem -days 36500 -subj "/CN=test-idp.example"
  openssl req -x509 -newkey rsa:2048 -nodes -keyout attacker_key.pem -out attacker_cert.pem -days 36500 -subj "/CN=attacker.example"
  python gen_fixtures.py   (needs `pip install signxml`), then delete the *_key.pem files.
"""
from lxml import etree
from signxml import XMLSigner, methods, SignatureConstructionMethod, CanonicalizationMethod
import copy

SAMLP = "urn:oasis:names:tc:SAML:2.0:protocol"
SAML = "urn:oasis:names:tc:SAML:2.0:assertion"

RESPONSE = f"""<samlp:Response xmlns:samlp="{SAMLP}" xmlns:saml="{SAML}" ID="_resp1" Version="2.0" IssueInstant="2026-01-01T00:00:00Z" Destination="https://app.example/acs">
<saml:Issuer>https://idp.example</saml:Issuer>
<samlp:Status><samlp:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"/></samlp:Status>
<saml:Assertion ID="_assert1" Version="2.0" IssueInstant="2026-01-01T00:00:00Z">
<saml:Issuer>https://idp.example</saml:Issuer>
<saml:Subject><saml:NameID Format="urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress">alice@acme.example</saml:NameID></saml:Subject>
<saml:Conditions NotBefore="2026-01-01T00:00:00Z" NotOnOrAfter="2026-01-01T00:05:00Z"/>
<saml:AuthnStatement AuthnInstant="2026-01-01T00:00:00Z" SessionIndex="_sess1"><saml:AuthnContext><saml:AuthnContextClassRef>urn:oasis:names:tc:SAML:2.0:ac:classes:Password</saml:AuthnContextClassRef></saml:AuthnContext></saml:AuthnStatement>
<saml:AttributeStatement>
<saml:Attribute Name="givenName"><saml:AttributeValue>Alice</saml:AttributeValue></saml:Attribute>
<saml:Attribute Name="surname"><saml:AttributeValue>Anderson</saml:AttributeValue></saml:Attribute>
</saml:AttributeStatement>
</saml:Assertion>
</samlp:Response>"""

key = open("idp_key.pem", "rb").read()
cert = open("idp_cert.pem", "rb").read()

def signer(prefix=None):
    s = XMLSigner(method=methods.enveloped, signature_algorithm="rsa-sha256", digest_algorithm="sha256",
                  c14n_algorithm=CanonicalizationMethod.EXCLUSIVE_XML_CANONICALIZATION_1_0)
    if prefix is not None:
        s.namespaces = {prefix: "http://www.w3.org/2000/09/xmldsig#"}
    return s

def write(name, root):
    open(name, "wb").write(etree.tostring(root))

def assertion_of(root):
    return root.find(f"{{{SAML}}}Assertion")

# 1. assertion signed (ds: prefix)
r = etree.fromstring(RESPONSE)
a = assertion_of(r)
signed = signer().sign(a, key=key, cert=cert, reference_uri="_assert1")
r.replace(a, signed)
write("response_assertion_signed.xml", r)

# 2. response signed
r = etree.fromstring(RESPONSE)
signed = signer().sign(r, key=key, cert=cert, reference_uri="_resp1")
write("response_response_signed.xml", signed)

# 3. assertion signed with default-namespace dsig prefix
r = etree.fromstring(RESPONSE)
a = assertion_of(r)
signed = signer(prefix="dsig").sign(a, key=key, cert=cert, reference_uri="_assert1")
r.replace(a, signed)
write("response_assertion_signed_dsigprefix.xml", r)

# 4. signed by an attacker key (valid signature, wrong signer)
r = etree.fromstring(RESPONSE)
a = assertion_of(r)
signed = signer().sign(a, key=open("attacker_key.pem", "rb").read(), cert=open("attacker_cert.pem", "rb").read(), reference_uri="_assert1")
r.replace(a, signed)
write("response_attacker_signed.xml", r)

# 5. unsigned
write("response_unsigned.xml", etree.fromstring(RESPONSE))

# 6. different SAML prefixes (saml2 / saml2p), assertion signed
alt = RESPONSE.replace("samlp:", "saml2p:").replace("xmlns:samlp", "xmlns:saml2p").replace("saml:", "saml2:").replace("xmlns:saml=", "xmlns:saml2=")
r = etree.fromstring(alt)
a = assertion_of(r)
signed = signer().sign(a, key=key, cert=cert, reference_uri="_assert1")
r.replace(a, signed)
write("response_assertion_signed_saml2prefix.xml", r)

# 7. valid signature over the Response Issuer only (does not cover the Assertion)
r = etree.fromstring(RESPONSE)
iss = r.find(f"{{{SAML}}}Issuer")
iss.set("ID", "_iss1")
signed = signer().sign(iss, key=key, cert=cert, reference_uri="_iss1")
r.replace(iss, signed)
write("response_issuer_signed_only.xml", r)

# Hand-built signatures (lxml c14n + cryptography RSA), for cases signxml cannot produce.
import base64, hashlib
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import padding

DS = "http://www.w3.org/2000/09/xmldsig#"
EXC = "http://www.w3.org/2001/10/xml-exc-c14n#"
ENVELOPED = f'<ds:Transform Algorithm="{DS}enveloped-signature"/>'
EXC_T = f'<ds:Transform Algorithm="{EXC}"/>'
XPATH = '<ds:Transform Algorithm="http://www.w3.org/TR/1999/REC-xpath-19991116"><ds:XPath xmlns:ds="%s">not(ancestor-or-self::ds:Signature)</ds:XPath></ds:Transform>' % DS
# Transform element in a FOREIGN namespace (default xmlns), XPath child in dsig ns.
XPATH_FOREIGN_NS = '<Transform xmlns="urn:example:not-dsig" Algorithm="http://www.w3.org/TR/1999/REC-xpath-19991116"><ds:XPath xmlns:ds="%s">not(ancestor-or-self::ds:Signature)</ds:XPath></Transform>' % DS

def hand_sign(target, uri, transforms_xml):
    """Append an enveloped ds:Signature to `target` (digest taken before appending)."""
    digest = base64.b64encode(hashlib.sha256(etree.tostring(target, method="c14n", exclusive=True)).digest()).decode()
    sig = etree.fromstring(f"""<ds:Signature xmlns:ds="{DS}"><ds:SignedInfo><ds:CanonicalizationMethod Algorithm="{EXC}"/><ds:SignatureMethod Algorithm="http://www.w3.org/2001/04/xmldsig-more#rsa-sha256"/><ds:Reference URI="{uri}"><ds:Transforms>{transforms_xml}</ds:Transforms><ds:DigestMethod Algorithm="http://www.w3.org/2001/04/xmlenc#sha256"/><ds:DigestValue>{digest}</ds:DigestValue></ds:Reference></ds:SignedInfo><ds:SignatureValue></ds:SignatureValue></ds:Signature>""")
    si = sig.find(f"{{{DS}}}SignedInfo")
    pk = serialization.load_pem_private_key(key, None)
    sv = base64.b64encode(pk.sign(etree.tostring(si, method="c14n", exclusive=True), padding.PKCS1v15(), hashes.SHA256())).decode()
    sig.find(f"{{{DS}}}SignatureValue").text = sv
    target.append(sig)

# 8. XPath transform (valid signature; must be rejected by the transform allowlist)
r = etree.fromstring(RESPONSE)
hand_sign(assertion_of(r), "#_assert1", ENVELOPED + XPATH + EXC_T)
write("response_assertion_xpath_transform.xml", r)

# 9. XPath transform whose Transform element is in a foreign namespace
r = etree.fromstring(RESPONSE)
hand_sign(assertion_of(r), "#_assert1", ENVELOPED + XPATH_FOREIGN_NS + EXC_T)
write("response_assertion_xpath_foreign_ns.xml", r)

# 10. Response signed with URI="" (whole-document enveloped), the most common real-IdP style
r = etree.fromstring(RESPONSE)
hand_sign(r, "", ENVELOPED + EXC_T)
write("response_response_signed_empty_uri.xml", r)
