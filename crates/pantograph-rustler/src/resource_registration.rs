use rustler::Env;

use crate::{
    ExtensionsResource, InferenceGatewayResource, NodeRegistryResource, OrchestrationStoreResource,
    PumasApiResource, WorkflowExecutorResource,
};

// Rustler 0.36 expands `resource!` into impl blocks that trigger Rust's
// `non_local_definitions` lint when invoked inside the load-time registration
// function. Keep this scoped to the macro boundary and remove it when Rustler
// exposes a warning-clean registration API for these resources.
#[allow(non_local_definitions)]
pub(crate) fn register_resources(env: Env) {
    let _ = rustler::resource!(WorkflowExecutorResource, env);
    let _ = rustler::resource!(OrchestrationStoreResource, env);
    let _ = rustler::resource!(NodeRegistryResource, env);
    let _ = rustler::resource!(PumasApiResource, env);
    let _ = rustler::resource!(ExtensionsResource, env);
    let _ = rustler::resource!(InferenceGatewayResource, env);
}
