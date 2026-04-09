# mockforge-registry-core

Shared domain models, storage trait, and OSS-safe handlers used by both the
MockForge SaaS registry server and the OSS admin UI embedded in
`mockforge-ui`.

This crate is backend-agnostic via the [`RegistryStore`] trait. Enable the
`postgres` feature for the multi-tenant SaaS binary, or the `sqlite` feature
for the single-tenant OSS admin UI.
