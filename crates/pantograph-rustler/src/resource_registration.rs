use rustler::Env;

use crate::{
    ExtensionsResource, InferenceGatewayResource, NodeRegistryResource, OrchestrationStoreResource,
    PumasApiResource, WorkflowExecutorResource,
};

pub(crate) fn register_resources(env: Env) {
    let _ = env.register::<WorkflowExecutorResource>();
    let _ = env.register::<OrchestrationStoreResource>();
    let _ = env.register::<NodeRegistryResource>();
    let _ = env.register::<PumasApiResource>();
    let _ = env.register::<ExtensionsResource>();
    let _ = env.register::<InferenceGatewayResource>();
}
