// Generated. Do not edit.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GreetInput {
#[serde(rename = "name")]
pub r#name: String,
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GreetOutput {
#[serde(rename = "message")]
pub r#message: String,
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvalidNameData {
#[serde(rename = "reason")]
pub r#reason: String,
}
pub const INTERFACE_ID: &str = "example.greeter";
pub const INTERFACE_VERSION: &str = "1.0.0";
pub const CONTRACT_DIGEST: &str = "ca8aa1104513e5324f7aaaca699f3c4160f4b35ba934b4c1fc3d236aa77c9824";
pub const CONTRACT_JSON: &str = "{\"descriptor\":{\"descriptorVersion\":1,\"errors\":{\"INVALID_NAME\":{\"data\":\"#/$defs/InvalidNameData\"}},\"events\":{},\"id\":\"example.greeter\",\"methods\":{\"greet\":{\"errors\":[\"INVALID_NAME\"],\"idempotent\":true,\"input\":\"#/$defs/GreetInput\",\"output\":\"#/$defs/GreetOutput\"}},\"version\":\"1.0.0\"},\"digest\":\"ca8aa1104513e5324f7aaaca699f3c4160f4b35ba934b4c1fc3d236aa77c9824\",\"schemas\":{\"$defs\":{\"GreetInput\":{\"additionalProperties\":false,\"properties\":{\"name\":{\"maxLength\":200,\"minLength\":1,\"type\":\"string\"}},\"required\":[\"name\"],\"type\":\"object\"},\"GreetOutput\":{\"additionalProperties\":false,\"properties\":{\"message\":{\"maxLength\":1024,\"type\":\"string\"}},\"required\":[\"message\"],\"type\":\"object\"},\"InvalidNameData\":{\"additionalProperties\":false,\"properties\":{\"reason\":{\"maxLength\":200,\"type\":\"string\"}},\"required\":[\"reason\"],\"type\":\"object\"}}}}";

pub fn contract() -> napi_vm_plugin_protocol::PluginResult<napi_vm_plugin_protocol::Contract> { napi_vm_plugin_protocol::Contract::from_value(serde_json::from_str(CONTRACT_JSON)?) }
pub fn validate_greet_input(value: serde_json::Value) -> napi_vm_plugin_protocol::PluginResult<GreetInput> { contract()?.validate_ref("#/$defs/GreetInput", &value)?; napi_vm_plugin_protocol::from_wire_value(value) }
pub fn validate_greet_output(value: serde_json::Value) -> napi_vm_plugin_protocol::PluginResult<GreetOutput> { contract()?.validate_ref("#/$defs/GreetOutput", &value)?; napi_vm_plugin_protocol::from_wire_value(value) }
pub fn validate_invalid_name_data(value: serde_json::Value) -> napi_vm_plugin_protocol::PluginResult<InvalidNameData> { contract()?.validate_ref("#/$defs/InvalidNameData", &value)?; napi_vm_plugin_protocol::from_wire_value(value) }
#[derive(Clone)]
pub struct Client { pub context: napi_vm_plugin_sdk::CallContext }
impl Client {
pub async fn r#greet(&self, input: GreetInput) -> napi_vm_plugin_protocol::PluginResult<GreetOutput> { let c=contract()?; let input=serde_json::to_value(input)?; c.validate_ref("#/$defs/GreetInput", &input)?; let value = self.context.invoke(&c, "greet", input).await?; c.output("greet", &value)?; napi_vm_plugin_protocol::from_wire_value(value) }
}
pub trait Handler: Send + Sync + 'static {
fn r#greet(&self, input: GreetInput, context: napi_vm_plugin_sdk::CallContext) -> napi_vm_plugin_sdk::PluginFuture<'_, GreetOutput>;
}
pub fn register(registry: &napi_vm_plugin_sdk::Registry, handler: std::sync::Arc<dyn Handler>) -> napi_vm_plugin_protocol::PluginResult<()> {
let h=handler.clone(); registry.register(contract()?, "greet", move |value, context| { let h=h.clone(); Box::pin(async move { contract()?.validate_ref("#/$defs/GreetInput", &value)?; let output=serde_json::to_value(h.r#greet(napi_vm_plugin_protocol::from_wire_value(value)?, context).await?)?; contract()?.output("greet", &output)?; Ok(output) }) })?;
Ok(()) }
pub fn error_invalid_name(data: InvalidNameData, message: impl Into<String>) -> napi_vm_plugin_protocol::PluginResult<napi_vm_plugin_protocol::RpcError> { let data=serde_json::to_value(data)?; contract()?.validate_ref("#/$defs/InvalidNameData", &data)?; Ok(napi_vm_plugin_protocol::RpcError::domain("INVALID_NAME", message, data)) }
