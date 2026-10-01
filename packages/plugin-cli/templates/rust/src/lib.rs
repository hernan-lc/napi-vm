pub mod generated { include!("generated.rs"); }
use napi_vm_plugin_sdk::{CallContext, Plugin, PluginFuture, PluginResult, Registry};
use std::sync::Arc;
pub struct Greeter;
impl generated::Handler for Greeter {
    fn greet(&self, input: generated::GreetInput, context: CallContext) -> PluginFuture<'_, generated::GreetOutput> {
        Box::pin(async move { context.throw_if_cancelled()?; Ok(generated::GreetOutput { message: greeting(&input.name) }) })
    }
}
pub fn greeting(name: &str) -> String { format!("Hola, {name}") }
pub fn plugin() -> PluginResult<Plugin> {
    let registry = Registry::new();
    generated::register(&registry, Arc::new(Greeter))?;
    Ok(Plugin::new(registry))
}
#[cfg(test)]
mod tests {
    #[test] fn greets_unicode() { assert_eq!(super::greeting("Ana 🌍"), "Hola, Ana 🌍"); }
    #[tokio::test] async fn harness_runs_generated_contract() {
        let harness=napi_vm_plugin_sdk::Harness::new(super::plugin().unwrap(), napi_vm_plugin_sdk::Registry::new()).unwrap();
        harness.initialize(serde_json::Value::Null,serde_json::Value::Null).await.unwrap();
        let output=harness.invoke(&super::generated::contract().unwrap(), "greet", serde_json::json!({"name":"Ana"})).await.unwrap();
        assert_eq!(output,serde_json::json!({"message":"Hola, Ana"}));
        harness.shutdown().await.unwrap();
    }
}
