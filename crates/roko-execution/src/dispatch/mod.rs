//! Agent dispatch bundle — shared factory, model resolver, and request types.
//!
//! This module consolidates the reusable dispatch components that were
//! previously rebuilt independently by runner-v2, Graph, workflow, and
//! chat entry points. The canonical construction path lives in
//! [`RuntimeServicesBuilder`](crate::builder::RuntimeServicesBuilder).

pub mod factory;
pub mod model_resolver;
pub mod request;

pub use factory::DispatchFactory;
pub use model_resolver::ModelResolverHandle;
pub use request::DispatchRequest;
