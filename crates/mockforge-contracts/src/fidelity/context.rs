//! Context-sensitive metadata filtering and schema-set normalization.

#[derive(Clone, Copy)]
pub(super) enum Context {
    Contract,
    Schema,
    Contracts,
    Schemas,
    Literal,
}

impl Context {
    pub(super) fn child(self, key: &str) -> Option<Self> {
        match self {
            Self::Contracts => Some(Self::Contract),
            Self::Schemas => Some(Self::Schema),
            Self::Literal => Some(Self::Literal),
            Self::Schema => match key {
                "description" | "title" | "example" | "examples" | "$comment" => None,
                "properties" | "patternProperties" | "$defs" | "definitions" => Some(Self::Schemas),
                "default" | "enum" | "const" => Some(Self::Literal),
                _ => Some(Self::Schema),
            },
            Self::Contract => match key {
                "description" | "summary" | "externalDocs" | "example" | "examples"
                | "operationId" | "tags" => None,
                "schema" => Some(Self::Schema),
                "content" | "responses" | "headers" | "encoding" => Some(Self::Contracts),
                "default" | "security" => Some(Self::Literal),
                _ => Some(Self::Contract),
            },
        }
    }

    pub(super) fn is_set(self, key: &str) -> bool {
        matches!(self, Self::Schema) && matches!(key, "required" | "enum")
    }
}
