#[tokio::main]
async fn main() -> napi_vm_plugin_sdk::PluginResult<()> {
    napi_vm_plugin_sdk::serve(trusted_greeter_template::plugin()?, napi_vm_plugin_sdk::PluginMetadata {
        id: "example.greeter".into(), version: "0.1.0".into(), requires_host: vec![],
    }).await
}
