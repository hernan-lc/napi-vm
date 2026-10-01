//! Shared contract and cross-runtime conformance entry points.
#![forbid(unsafe_code)]
#[cfg(test)]
mod tests {
    use napi_vm_plugin_protocol::*;
    use serde_json::{Value, json};
    #[allow(dead_code)]
    mod wire {
        include!("../../../contracts/trusted-plugins/generated/wire-types.rs");
    }
    #[test]
    fn shared_validation_fixtures() {
        let cases: Vec<Value> = serde_json::from_str(include_str!(
            "../../../fixtures/trusted-plugins/contracts/validation.json"
        ))
        .unwrap();
        for case in cases {
            let result = validate(&case["schema"], &case["value"]);
            assert_eq!(
                result.is_ok(),
                case["valid"].as_bool().unwrap(),
                "{}: {:?}",
                case["name"],
                result
            );
        }
    }
    #[test]
    fn generated_presence_and_normalized_numbers() {
        let value = json!({"__proto__":"data","choice":"🌍","created":"2024-02-29T00:00:00.000Z","data":"YQ==","integer":1.0,"literal":1.0,"tagged":{"kind":"text","value":"ok"},"wide":"18446744073709551615"});
        let parsed = wire::validate_wire_types(value.clone()).unwrap();
        assert_eq!(parsed.integer, 1);
        assert!(parsed.optional_null.is_missing());
        let mut normalized = value.clone();
        normalize_value(&mut normalized, 0, 64).unwrap();
        assert_eq!(serde_json::to_value(&parsed).unwrap(), normalized);
        let mut nullable = value;
        nullable["optionalNull"] = Value::Null;
        assert_eq!(
            wire::validate_wire_types(nullable).unwrap().optional_null,
            Field::Value(None)
        );
    }
    #[test]
    fn generated_literal_escape_roundtrips() {
        assert_eq!(
            serde_json::to_value(wire::WireTypesChoice::V3).unwrap(),
            json!(r"literal\u0010")
        );
        assert_eq!(
            serde_json::to_value(wire::WireTypesChoice::V4).unwrap(),
            json!("controls\u{8}\u{c}")
        );
    }
}
