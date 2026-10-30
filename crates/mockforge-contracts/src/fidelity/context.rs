//! Context-sensitive metadata filtering and schema-set normalization.

#[derive(Clone, Copy)]
pub(super) enum Context {
    Contract,
    AsyncContract,
    AsyncContracts,
    Bindings,
    Binding,
    Schema,
    Contracts,
    Schemas,
    Literal,
}

impl Context {
    pub(super) fn child(self, key: &str) -> Option<Self> {
        match self {
            Self::AsyncContracts => Some(Self::AsyncContract),
            Self::Bindings => Some(Self::Binding),
            Self::Binding => Some(match key {
                "query" | "headers" | "key" | "groupId" | "clientId" => Self::Schema,
                _ => Self::Literal,
            }),
            Self::AsyncContract => match key {
                "description" | "title" | "summary" | "externalDocs" | "example" | "examples"
                | "operationId" | "tags" => None,
                "payload" | "headers" | "schema" => Some(Self::Schema),
                "messages" | "parameters" => Some(Self::AsyncContracts),
                "bindings" => Some(Self::Bindings),
                "security" => Some(Self::Literal),
                _ => Some(Self::AsyncContract),
            },
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
