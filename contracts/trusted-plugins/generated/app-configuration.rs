// Generated. Do not edit.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetInput {
#[serde(rename = "key")]
pub r#key: String,
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetOutput {
#[serde(rename = "value")]
pub r#value: String,
}
pub const INTERFACE_ID: &str = "app.configuration";
pub const INTERFACE_VERSION: &str = "1.0.0";
pub const CONTRACT_DIGEST: &str = "db06a9046edec0cff9afe336ca7bea7011133ac037f87dff207f652d2b47113f";
pub const CONTRACT_JSON: &str = "{\"descriptor\":{\"descriptorVersion\":1,\"errors\":{},\"events\":{},\"id\":\"app.configuration\",\"methods\":{\"get\":{\"errors\":[],\"input\":\"#/$defs/GetInput\",\"output\":\"#/$defs/GetOutput\"}},\"version\":\"1.0.0\"},\"digest\":\"db06a9046edec0cff9afe336ca7bea7011133ac037f87dff207f652d2b47113f\",\"schemas\":{\"$defs\":{\"GetInput\":{\"additionalProperties\":false,\"properties\":{\"key\":{\"maxLength\":200,\"type\":\"string\"}},\"required\":[\"key\"],\"type\":\"object\"},\"GetOutput\":{\"additionalProperties\":false,\"properties\":{\"value\":{\"maxLength\":4096,\"type\":\"string\"}},\"required\":[\"value\"],\"type\":\"object\"}}}}";

pub fn contract() -> napi_vm_plugin_protocol::PluginResult<napi_vm_plugin_protocol::Contract> { napi_vm_plugin_protocol::Contract::from_value(serde_json::from_str(CONTRACT_JSON)?) }
pub fn validate_get_input(value: serde_json::Value) -> napi_vm_plugin_protocol::PluginResult<GetInput> { contract()?.validate_ref("#/$defs/GetInput", &value)?; napi_vm_plugin_protocol::from_wire_value(value) }
pub fn validate_get_output(value: serde_json::Value) -> napi_vm_plugin_protocol::PluginResult<GetOutput> { contract()?.validate_ref("#/$defs/GetOutput", &value)?; napi_vm_plugin_protocol::from_wire_value(value) }
#[derive(Clone)]
pub struct Client { pub context: napi_vm_plugin_sdk::CallContext }
impl Client {
pub async fn r#get(&self, input: GetInput) -> napi_vm_plugin_protocol::PluginResult<GetOutput> { let c=contract()?; let input=serde_json::to_value(input)?; c.validate_ref("#/$defs/GetInput", &input)?; let value = self.context.invoke(&c, "get", input).await?; c.output("get", &value)?; napi_vm_plugin_protocol::from_wire_value(value) }
}
pub trait Handler: Send + Sync + 'static {
fn r#get(&self, input: GetInput, context: napi_vm_plugin_sdk::CallContext) -> napi_vm_plugin_sdk::PluginFuture<'_, GetOutput>;
}
pub fn register(registry: &napi_vm_plugin_sdk::Registry, handler: std::sync::Arc<dyn Handler>) -> napi_vm_plugin_protocol::PluginResult<()> {
let h=handler.clone(); registry.register(contract()?, "get", move |value, context| { let h=h.clone(); Box::pin(async move { contract()?.validate_ref("#/$defs/GetInput", &value)?; let output=serde_json::to_value(h.r#get(napi_vm_plugin_protocol::from_wire_value(value)?, context).await?)?; contract()?.output("get", &output)?; Ok(output) }) })?;
Ok(()) }
