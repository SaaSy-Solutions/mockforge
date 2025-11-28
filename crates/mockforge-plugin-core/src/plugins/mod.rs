//! Built-in client generator plugins
//!
//! This module contains the default client generator plugins that come
//! with MockForge for popular frontend frameworks.

pub mod angular_client_generator;
pub mod react_client_generator;
pub mod svelte_client_generator;
pub mod vue_client_generator;

pub use angular_client_generator::AngularClientGenerator;
pub use react_client_generator::ReactClientGenerator;
pub use svelte_client_generator::SvelteClientGenerator;
pub use vue_client_generator::VueClientGenerator;
