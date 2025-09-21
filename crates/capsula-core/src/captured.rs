use serde_json::Value;

pub trait Captured {
    fn to_json(&self) -> Value;

    fn abort_requested(&self) -> bool {
        false
    }
}
