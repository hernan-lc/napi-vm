use napi_vm_plugin_sdk::{PluginMetadata, serve};
#[tokio::main]
async fn main() {
    let result = async {
        let metadata: PluginMetadata = serde_json::from_str(include_str!("plugin-metadata.json"))?;
        serve(trusted_counter_rust::plugin()?, metadata).await
    }
    .await;
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
