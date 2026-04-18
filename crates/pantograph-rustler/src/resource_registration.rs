use rustler::Env;

use crate::{
    ExtensionsResource, InferenceGatewayResource, NodeRegistryResource, OrchestrationStoreResource,
    PumasApiResource, WorkflowExecutorResource,
};

pub(crate) fn register_resources(env: Env) {
    let _ = rustler::resource!(WorkflowExecutorResource, env);
    let _ = rustler::resource!(OrchestrationStoreResource, env);
    let _ = rustler::resource!(NodeRegistryResource, env);
    let _ = rustler::resource!(PumasApiResource, env);
    let _ = rustler::resource!(ExtensionsResource, env);
    let _ = rustler::resource!(InferenceGatewayResource, env);
}
