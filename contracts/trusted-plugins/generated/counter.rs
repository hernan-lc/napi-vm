// Generated. Do not edit.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddInput {
#[serde(rename = "amount")]
pub r#amount: i64,
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Count {
#[serde(rename = "count")]
pub r#count: String,
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Empty {

}
pub const INTERFACE_ID: &str = "example.counter";
pub const INTERFACE_VERSION: &str = "1.0.0";
pub const CONTRACT_DIGEST: &str = "a6cc7c1e4c4ceedaaa1dde5626c052d5320c1b26ee21c63e3d581d4d972fc90f";
pub const CONTRACT_JSON: &str = "{\"descriptor\":{\"descriptorVersion\":1,\"errors\":{},\"events\":{\"changed\":{\"payload\":\"#/$defs/Count\"}},\"id\":\"example.counter\",\"methods\":{\"add\":{\"errors\":[],\"input\":\"#/$defs/AddInput\",\"output\":\"#/$defs/Count\"},\"get\":{\"errors\":[],\"idempotent\":true,\"input\":\"#/$defs/Empty\",\"output\":\"#/$defs/Count\"}},\"state\":{\"contract\":\"example.counter.state\",\"schema\":\"#/$defs/Count\",\"version\":1},\"version\":\"1.0.0\"},\"digest\":\"a6cc7c1e4c4ceedaaa1dde5626c052d5320c1b26ee21c63e3d581d4d972fc90f\",\"schemas\":{\"$defs\":{\"AddInput\":{\"additionalProperties\":false,\"properties\":{\"amount\":{\"maximum\":1000000,\"minimum\":-1000000,\"type\":\"integer\"}},\"required\":[\"amount\"],\"type\":\"object\"},\"Count\":{\"additionalProperties\":false,\"properties\":{\"count\":{\"type\":\"string\",\"x-wire-type\":\"i64-decimal\"}},\"required\":[\"count\"],\"type\":\"object\"},\"Empty\":{\"additionalProperties\":false,\"properties\":{},\"required\":[],\"type\":\"object\"}}}}";

pub fn contract() -> napi_vm_plugin_protocol::PluginResult<napi_vm_plugin_protocol::Contract> { napi_vm_plugin_protocol::Contract::from_value(serde_json::from_str(CONTRACT_JSON)?) }
pub fn validate_add_input(value: serde_json::Value) -> napi_vm_plugin_protocol::PluginResult<AddInput> { contract()?.validate_ref("#/$defs/AddInput", &value)?; napi_vm_plugin_protocol::from_wire_value(value) }
pub fn validate_count(value: serde_json::Value) -> napi_vm_plugin_protocol::PluginResult<Count> { contract()?.validate_ref("#/$defs/Count", &value)?; napi_vm_plugin_protocol::from_wire_value(value) }
pub fn validate_empty(value: serde_json::Value) -> napi_vm_plugin_protocol::PluginResult<Empty> { contract()?.validate_ref("#/$defs/Empty", &value)?; napi_vm_plugin_protocol::from_wire_value(value) }
#[derive(Clone)]
pub struct Client { pub context: napi_vm_plugin_sdk::CallContext }
impl Client {
pub async fn r#add(&self, input: AddInput) -> napi_vm_plugin_protocol::PluginResult<Count> { let c=contract()?; let input=serde_json::to_value(input)?; c.validate_ref("#/$defs/AddInput", &input)?; let value = self.context.invoke(&c, "add", input).await?; c.output("add", &value)?; napi_vm_plugin_protocol::from_wire_value(value) }
pub async fn r#get(&self, input: Empty) -> napi_vm_plugin_protocol::PluginResult<Count> { let c=contract()?; let input=serde_json::to_value(input)?; c.validate_ref("#/$defs/Empty", &input)?; let value = self.context.invoke(&c, "get", input).await?; c.output("get", &value)?; napi_vm_plugin_protocol::from_wire_value(value) }
}
pub trait Handler: Send + Sync + 'static {
fn r#add(&self, input: AddInput, context: napi_vm_plugin_sdk::CallContext) -> napi_vm_plugin_sdk::PluginFuture<'_, Count>;
fn r#get(&self, input: Empty, context: napi_vm_plugin_sdk::CallContext) -> napi_vm_plugin_sdk::PluginFuture<'_, Count>;
}
pub fn register(registry: &napi_vm_plugin_sdk::Registry, handler: std::sync::Arc<dyn Handler>) -> napi_vm_plugin_protocol::PluginResult<()> {
let h=handler.clone(); registry.register(contract()?, "add", move |value, context| { let h=h.clone(); Box::pin(async move { contract()?.validate_ref("#/$defs/AddInput", &value)?; let output=serde_json::to_value(h.r#add(napi_vm_plugin_protocol::from_wire_value(value)?, context).await?)?; contract()?.output("add", &output)?; Ok(output) }) })?;
let h=handler.clone(); registry.register(contract()?, "get", move |value, context| { let h=h.clone(); Box::pin(async move { contract()?.validate_ref("#/$defs/Empty", &value)?; let output=serde_json::to_value(h.r#get(napi_vm_plugin_protocol::from_wire_value(value)?, context).await?)?; contract()?.output("get", &output)?; Ok(output) }) })?;
Ok(()) }
