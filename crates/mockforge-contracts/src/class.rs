//! A weakness in the terms other security tools use: a CWE, a weakness in
//! MITRE's list, and a category of a list of OWASP's. A flaw, an injection
//! and a bait of a range pack, and a rule of an agent red-team suite, may name
//! theirs; the SARIF and the JUnit carry them to the tools that group and
//! filter by them.

use serde_json::{json, Value};

/// Whether `text` names a weakness of MITRE's list as its site writes it:
/// `CWE-89`, the letters upper case and the number bare.
pub fn is_cwe(text: &str) -> bool {
    text.strip_prefix("CWE-")
        .is_some_and(|number| digits(number) && !number.is_empty())
}

/// Whether `text` names a category of a list of OWASP's as OWASP writes it: a
/// family, its number, a colon and a year. `A03:2021` is the Top 10, `API1:2023`
/// the API Security Top 10 and `LLM01:2025` the LLM Top 10.
pub fn is_owasp(text: &str) -> bool {
    let Some((family, year)) = text.split_once(':') else {
        return false;
    };
    let name = family.chars().take_while(char::is_ascii_uppercase).count();
    let number = family.get(name..).unwrap_or_default();
    name > 0
        && !number.is_empty()
        && number.len() <= 2
        && digits(number)
        && year.len() == 4
        && digits(year)
}

/// Whether `text` is digits, and nothing else.
fn digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// What is wrong with a class, when something is: an id that is not written as
/// its list writes it. `what` completes "the flaw `id` has …".
pub fn problem(cwe: Option<&str>, owasp: Option<&str>) -> Option<String> {
    let named = [
        ("cwe", cwe, is_cwe as fn(&str) -> bool, "`CWE-89`"),
        ("owasp", owasp, is_owasp as fn(&str) -> bool, "`A03:2021`"),
    ];
    named.iter().find_map(|(name, text, well_formed, shape)| {
        text.filter(|text| !well_formed(text))
            .map(|text| format!("`{name}` set to `{text}`, which is not written as {shape}"))
    })
}

/// The class in a line of text: both ids, the CWE first, or nothing when
/// neither is named.
pub fn label(cwe: Option<&str>, owasp: Option<&str>) -> String {
    let mut parts: Vec<&str> = Vec::new();
    parts.extend(cwe);
    parts.extend(owasp);
    parts.join(" ")
}

/// The tag by which a SARIF tool knows a CWE, as CodeQL and LGTM write it and
/// GitHub code scanning filters on it: `external/cwe/cwe-89`.
fn cwe_tag(cwe: &str) -> String {
    format!("external/cwe/{}", cwe.to_ascii_lowercase())
}

/// The tags of a SARIF rule of this class, written under the rule's
/// `properties` as `tags`, which is where code scanning reads them: a rule of
/// the schema has no `tags` of its own. They are `security`, the CWE's tag,
/// and the OWASP category as it is written, since there is no one way tools
/// tag one.
pub fn tag(properties: &mut Value, cwe: Option<&str>, owasp: Option<&str>) {
    let mut tags = vec![Value::String("security".to_string())];
    tags.extend(cwe.map(|cwe| Value::String(cwe_tag(cwe))));
    tags.extend(owasp.map(str::to_string).map(Value::String));
    properties["tags"] = Value::Array(tags);
}

/// `cwe` and `owasp` as the properties of a SARIF rule or result carry them:
/// nothing for one that is not named.
pub fn properties(cwe: Option<&str>, owasp: Option<&str>) -> Value {
    let mut properties = Value::Object(serde_json::Map::new());
    add_to(&mut properties, cwe, owasp);
    properties
}

/// `cwe` and `owasp` added to `properties`, for those that are named.
pub fn add_to(properties: &mut Value, cwe: Option<&str>, owasp: Option<&str>) {
    for (name, text) in [("cwe", cwe), ("owasp", owasp)] {
        if let Some(text) = text {
            properties[name] = json!(text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cwe_is_written_as_its_site_writes_it() {
        for text in ["CWE-89", "CWE-1427", "CWE-5"] {
            assert!(is_cwe(text), "{text}");
        }
        for text in ["cwe-89", "CWE-", "CWE-89 Injection", "89", "CWE-089x", ""] {
            assert!(!is_cwe(text), "{text}");
        }
    }

    #[test]
    fn an_owasp_category_is_a_family_a_number_and_a_year() {
        for text in [
            "A03:2021",
            "API1:2023",
            "LLM01:2025",
            "A1:2021",
            "API10:2023",
        ] {
            assert!(is_owasp(text), "{text}");
        }
        for text in [
            "a03:2021",
            "A03:2021 Injection",
            "A03 2021",
            "A03:21",
            "A03:20211",
            "A:2021",
            "A033:2021",
            "2021",
            "",
        ] {
            assert!(!is_owasp(text), "{text}");
        }
    }

    #[test]
    fn what_keeps_a_class_from_being_read_is_named() {
        assert_eq!(problem(None, None), None);
        assert_eq!(problem(Some("CWE-89"), None), None);
        assert_eq!(problem(None, Some("A03:2021")), None);
        assert_eq!(
            problem(Some("89"), Some("A03:2021")).as_deref(),
            Some("`cwe` set to `89`, which is not written as `CWE-89`")
        );
        // The first that is wrong is the one said.
        assert_eq!(
            problem(Some("CWE-89"), Some("injection")).as_deref(),
            Some("`owasp` set to `injection`, which is not written as `A03:2021`")
        );
    }

    #[test]
    fn the_class_in_a_line_and_in_the_tags_and_properties_of_sarif() {
        assert_eq!(label(None, None), "");
        assert_eq!(label(Some("CWE-89"), None), "CWE-89");
        assert_eq!(label(None, Some("A03:2021")), "A03:2021");
        assert_eq!(label(Some("CWE-89"), Some("A03:2021")), "CWE-89 A03:2021");

        // The tags of a rule are under its properties, where code scanning
        // reads them; the schema allows a rule no `tags` of its own.
        let tagged = |cwe: Option<&str>, owasp: Option<&str>| {
            let mut properties = properties(cwe, owasp);
            tag(&mut properties, cwe, owasp);
            properties
        };
        assert_eq!(tagged(None, None), json!({ "tags": ["security"] }));
        assert_eq!(
            tagged(Some("CWE-89"), None),
            json!({ "cwe": "CWE-89", "tags": ["security", "external/cwe/cwe-89"] })
        );
        assert_eq!(
            tagged(None, Some("A03:2021")),
            json!({ "owasp": "A03:2021", "tags": ["security", "A03:2021"] })
        );
        assert_eq!(
            tagged(Some("CWE-89"), Some("A03:2021")),
            json!({
                "cwe": "CWE-89",
                "owasp": "A03:2021",
                "tags": ["security", "external/cwe/cwe-89", "A03:2021"],
            })
        );

        assert_eq!(properties(None, None), json!({}));
        assert_eq!(
            properties(Some("CWE-89"), Some("A03:2021")),
            json!({ "cwe": "CWE-89", "owasp": "A03:2021" })
        );
        assert_eq!(properties(Some("CWE-89"), None), json!({ "cwe": "CWE-89" }));
    }
}
