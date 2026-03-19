use schemars::{schema_for, JsonSchema};
use serde_json::Value;

pub fn generate_schema<T: JsonSchema>() -> Value {
    let schema = schema_for!(T);
    serde_json::to_value(&schema).unwrap_or(Value::Null)
}

pub fn merge_schemas(base: &mut Value, additional: Value) {
    if let (Value::Object(base_obj), Value::Object(add_obj)) = (base, additional) {
        for (k, v) in add_obj {
            base_obj.insert(k, v);
        }
    }
}

pub fn add_description(schema: &mut Value, field: &str, description: &str) {
    if let Value::Object(obj) = schema {
        if let Some(Value::Object(properties)) = obj.get_mut("properties") {
            if let Some(Value::Object(field_obj)) = properties.get_mut(field) {
                field_obj.insert(
                    "description".to_string(),
                    Value::String(description.to_string()),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, JsonSchema)]
    struct TestParams {
        #[schemars(description = "The file path to read")]
        file_path: String,
        #[schemars(description = "Line offset (1-indexed)")]
        offset: Option<usize>,
    }

    #[test]
    fn test_generate_schema() {
        let schema = generate_schema::<TestParams>();
        assert!(schema.is_object());
        assert!(schema.get("properties").is_some());
    }
}
