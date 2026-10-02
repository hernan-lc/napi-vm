//! Business implementation uses generated contract adapters; no sockets on import.
use napi_vm_plugin_sdk::{CallContext, Plugin, PluginFuture, PluginResult, Registry, RpcError};
use serde_json::json;
use std::sync::Arc;
pub mod greeter {
    include!("../../../../contracts/trusted-plugins/generated/greeter.rs");
}
pub mod configuration {
    include!("../../../../contracts/trusted-plugins/generated/app-configuration.rs");
}
pub struct Greeter;
impl greeter::Handler for Greeter {
    fn greet(
        &self,
        input: greeter::GreetInput,
        context: CallContext,
    ) -> PluginFuture<'_, greeter::GreetOutput> {
        Box::pin(async move {
            context.throw_if_cancelled()?;
            if input.name == "invalid" {
                return Err(RpcError::domain(
                    "INVALID_NAME",
                    "The demonstration rejects this name",
                    json!({"reason":"The demonstration rejects this name"}),
                ));
            }
            let prefix = configuration::Client { context }
                .get(configuration::GetInput {
                    key: "greeting.prefix".into(),
                })
                .await?;
            Ok(greeter::GreetOutput {
                message: format!("{}, {}", prefix.value, input.name),
            })
        })
    }
}
pub fn plugin() -> PluginResult<Plugin> {
    let registry = Registry::new();
    greeter::register(&registry, Arc::new(Greeter))?;
    Ok(Plugin::new(registry))
}
#[cfg(test)]
mod tests {
    use super::*;
    use napi_vm_plugin_sdk::Harness;
    #[tokio::test]
    async fn callback_uses_same_contract() {
        let host = Registry::new();
        host.register(configuration::contract().unwrap(), "get", |_, _| {
            Box::pin(async { Ok(json!({"value":"Hola"})) })
        })
        .unwrap();
        let h = Harness::new(plugin().unwrap(), host).unwrap();
        h.initialize(json!({}), serde_json::Value::Null)
            .await
            .unwrap();
        let output = greeter::Client {
            context: h.context(),
        }
        .greet(greeter::GreetInput { name: "Ana".into() })
        .await
        .unwrap();
        assert_eq!(output.message, "Hola, Ana");
        assert_eq!(
            h.invoke(
                &greeter::contract().unwrap(),
                "greet",
                json!({"name":"invalid"})
            )
            .await
            .unwrap_err()
            .stable_code(),
            "APPLICATION_ERROR"
        );
        h.shutdown().await.unwrap();
    }
}
