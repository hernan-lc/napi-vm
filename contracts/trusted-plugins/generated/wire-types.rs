// Generated. Do not edit.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum WireTypesChoice { #[serde(rename = "🌍")] V0, #[serde(rename = "ready")] V1, #[serde(rename = "quote\"\\value")] V2, #[serde(rename = "literal\\u0010")] V3, #[serde(rename = "controls\u{8}\u{c}")] V4 }
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum WireTypesTaggedVariant0Kind { #[serde(rename = "text")] V0 }
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireTypesTaggedVariant0 {
#[serde(rename = "kind")]
pub r#kind: WireTypesTaggedVariant0Kind,
#[serde(rename = "value")]
pub r#value: String,
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum WireTypesTaggedVariant1Kind { #[serde(rename = "number")] V0 }
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireTypesTaggedVariant1 {
#[serde(rename = "kind")]
pub r#kind: WireTypesTaggedVariant1Kind,
#[serde(rename = "value")]
pub r#value: napi_vm_plugin_protocol::FiniteF64,
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum WireTypesTagged { V0(WireTypesTaggedVariant0), V1(WireTypesTaggedVariant1) }
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireTypes {
#[serde(rename = "__proto__")]
pub r#__proto__: String,
#[serde(rename = "choice")]
pub r#choice: WireTypesChoice,
#[serde(rename = "created")]
pub r#created: String,
#[serde(rename = "data")]
pub r#data: String,
#[serde(rename = "integer")]
pub r#integer: i64,
#[serde(rename = "literal")]
pub r#literal: i64,
#[serde(rename = "optionalNull", default, skip_serializing_if = "napi_vm_plugin_protocol::Field::is_missing")]
pub r#optional_null: napi_vm_plugin_protocol::Field<Option<String>>,
#[serde(rename = "optionalNumber", default, skip_serializing_if = "napi_vm_plugin_protocol::Field::is_missing")]
pub r#optional_number: napi_vm_plugin_protocol::Field<Option<napi_vm_plugin_protocol::FiniteF64>>,
#[serde(rename = "self", default, skip_serializing_if = "napi_vm_plugin_protocol::Field::is_missing")]
pub self_field: napi_vm_plugin_protocol::Field<Option<String>>,
#[serde(rename = "self-field", default, skip_serializing_if = "napi_vm_plugin_protocol::Field::is_missing")]
pub r#self_field_2: napi_vm_plugin_protocol::Field<String>,
#[serde(rename = "self_field", default, skip_serializing_if = "napi_vm_plugin_protocol::Field::is_missing")]
pub r#self_field_3: napi_vm_plugin_protocol::Field<String>,
#[serde(rename = "tagged")]
pub r#tagged: WireTypesTagged,
#[serde(rename = "wide")]
pub r#wide: String,
}
pub const INTERFACE_ID: &str = "example.wire-types";
pub const INTERFACE_VERSION: &str = "1.0.0";
pub const CONTRACT_DIGEST: &str = "a0cdc912547e345bc9def20c0bb4110b60908cbcf4178fb68bea1e72734f2a7b";
pub const CONTRACT_JSON: &str = "{\"descriptor\":{\"descriptorVersion\":1,\"errors\":{},\"events\":{},\"id\":\"example.wire-types\",\"methods\":{\"echo\":{\"errors\":[],\"input\":\"#/$defs/WireTypes\",\"output\":\"#/$defs/WireTypes\"}},\"version\":\"1.0.0\"},\"digest\":\"a0cdc912547e345bc9def20c0bb4110b60908cbcf4178fb68bea1e72734f2a7b\",\"schemas\":{\"$defs\":{\"WireTypes\":{\"additionalProperties\":false,\"properties\":{\"__proto__\":{\"type\":\"string\"},\"choice\":{\"enum\":[\"🌍\",\"ready\",\"quote\\\"\\\\value\",\"literal\\\\u0010\",\"controls\\b\\f\"],\"type\":\"string\"},\"created\":{\"type\":\"string\",\"x-wire-type\":\"utc-datetime\"},\"data\":{\"type\":\"string\",\"x-wire-type\":\"bytes-base64\"},\"integer\":{\"maximum\":9007199254740991,\"minimum\":-9007199254740991,\"type\":\"integer\"},\"literal\":{\"const\":1,\"type\":\"integer\"},\"optionalNull\":{\"type\":[\"string\",\"null\"]},\"optionalNumber\":{\"type\":[\"number\",\"null\"]},\"self\":{\"type\":[\"string\",\"null\"]},\"self-field\":{\"type\":\"string\"},\"self_field\":{\"type\":\"string\"},\"tagged\":{\"oneOf\":[{\"additionalProperties\":false,\"properties\":{\"kind\":{\"const\":\"text\",\"type\":\"string\"},\"value\":{\"type\":\"string\"}},\"required\":[\"kind\",\"value\"],\"type\":\"object\"},{\"additionalProperties\":false,\"properties\":{\"kind\":{\"const\":\"number\",\"type\":\"string\"},\"value\":{\"type\":\"number\"}},\"required\":[\"kind\",\"value\"],\"type\":\"object\"}]},\"wide\":{\"type\":\"string\",\"x-wire-type\":\"u64-decimal\"}},\"required\":[\"__proto__\",\"data\",\"wide\",\"created\",\"integer\",\"literal\",\"choice\",\"tagged\"],\"type\":\"object\"}}}}";

pub fn contract() -> napi_vm_plugin_protocol::PluginResult<napi_vm_plugin_protocol::Contract> { napi_vm_plugin_protocol::Contract::from_value(serde_json::from_str(CONTRACT_JSON)?) }
pub fn validate_wire_types(value: serde_json::Value) -> napi_vm_plugin_protocol::PluginResult<WireTypes> { contract()?.validate_ref("#/$defs/WireTypes", &value)?; napi_vm_plugin_protocol::from_wire_value(value) }
#[derive(Clone)]
pub struct Client { pub context: napi_vm_plugin_sdk::CallContext }
impl Client {
pub async fn r#echo(&self, input: WireTypes) -> napi_vm_plugin_protocol::PluginResult<WireTypes> { let c=contract()?; let input=serde_json::to_value(input)?; c.validate_ref("#/$defs/WireTypes", &input)?; let value = self.context.invoke(&c, "echo", input).await?; c.output("echo", &value)?; napi_vm_plugin_protocol::from_wire_value(value) }
}
pub trait Handler: Send + Sync + 'static {
fn r#echo(&self, input: WireTypes, context: napi_vm_plugin_sdk::CallContext) -> napi_vm_plugin_sdk::PluginFuture<'_, WireTypes>;
}
pub fn register(registry: &napi_vm_plugin_sdk::Registry, handler: std::sync::Arc<dyn Handler>) -> napi_vm_plugin_protocol::PluginResult<()> {
let h=handler.clone(); registry.register(contract()?, "echo", move |value, context| { let h=h.clone(); Box::pin(async move { contract()?.validate_ref("#/$defs/WireTypes", &value)?; let output=serde_json::to_value(h.r#echo(napi_vm_plugin_protocol::from_wire_value(value)?, context).await?)?; contract()?.output("echo", &output)?; Ok(output) }) })?;
Ok(()) }
