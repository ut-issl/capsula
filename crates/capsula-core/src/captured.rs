use serde::Serialize;
use serde_json::{Map, Value};

/// Output captured by a hook.
pub trait Captured {
    /// Serialize the output as a JSON object.
    ///
    /// The orchestrator stores hook metadata under a `__meta` key next to the
    /// output's fields, so the output must be an object. Implementations
    /// usually delegate to [`to_object`].
    fn serialize_json(&self) -> Result<Map<String, Value>, serde_json::Error>;
}

/// Serialize `value` as a JSON object, failing when it serializes to any
/// other JSON type (e.g. a string from a `#[serde(transparent)]` newtype).
pub fn to_object<T: Serialize + ?Sized>(
    value: &T,
) -> Result<Map<String, Value>, serde_json::Error> {
    match serde_json::to_value(value)? {
        Value::Object(map) => Ok(map),
        other => Err(serde::ser::Error::custom(format!(
            "captured output must serialize to a JSON object, got {}",
            json_type_name(&other)
        ))),
    }
}

const fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "Tests use unwrap for brevity")]

    use super::*;

    #[derive(Serialize)]
    struct Fields {
        cwd: String,
    }

    #[derive(Serialize)]
    #[serde(transparent)]
    struct Transparent(String);

    #[test]
    fn to_object_accepts_struct_with_fields() {
        let map = to_object(&Fields {
            cwd: "/p".to_string(),
        })
        .unwrap();
        assert_eq!(map["cwd"], "/p");
    }

    #[test]
    fn to_object_rejects_non_object() {
        let err = to_object(&Transparent("/p".to_string())).unwrap_err();
        assert!(err.to_string().contains("got a string"), "{err}");
        assert!(to_object(&vec![1, 2]).is_err());
    }
}
