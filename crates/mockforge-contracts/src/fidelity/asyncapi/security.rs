//! Security contract normalization.
use super::{traits, Context, Document};
use serde_json::{json, Map, Value};

impl Document<'_> {
    pub(super) fn security(&mut self, security: &Value) -> Result<Value, String> {
        let requirements = security.as_array().ok_or("AsyncAPI security must be an array")?;
        let mut out = Vec::new();
        for requirement in requirements {
            if self.major == 3 {
                out.push(self.security_scheme(requirement)?);
                continue;
            }
            let mut schemes = Map::new();
            for (name, scopes) in
                requirement.as_object().ok_or("Invalid AsyncAPI security requirement")?
            {
                let schema = self
                    .source
                    .get("components")
                    .and_then(|c| c.get("securitySchemes"))
                    .and_then(|s| s.get(name))
                    .ok_or("Undefined AsyncAPI security scheme")?;
                let mut scopes =
                    scopes.as_array().ok_or("Security scopes must be an array")?.clone();
                scopes.sort_by_key(Value::to_string);
                schemes.insert(
                    name.clone(),
                    json!({"scheme":self.security_scheme(schema)?,"scopes":scopes}),
                );
            }
            out.push(Value::Object(schemes));
        }
        out.sort_by_key(Value::to_string);
        Ok(Value::Array(out))
    }

    pub(super) fn security_scheme(&mut self, raw: &Value) -> Result<Value, String> {
        let mut scheme = self.projection.value(raw, Context::AsyncContract, 0)?;
        traits::apply_traits(&mut scheme)?;
        if scheme.get("type").and_then(Value::as_str).is_none() || scheme.get("security").is_some()
        {
            return Err(
                "AsyncAPI security scheme must declare a type without nested security".into()
            );
        }
        Ok(scheme)
    }
}
