pub trait Captured {
    fn serialize_json(&self) -> Result<serde_json::Value, serde_json::Error>;
}
