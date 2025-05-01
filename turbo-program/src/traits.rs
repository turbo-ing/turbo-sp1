pub trait TurboActionSerialization: Sized {
    fn deserialize(action: &[u8]) -> Result<(Self, &[u8]), &'static str>;
    fn serialize_json(json_str: &str) -> Result<Vec<u8>, &'static str>;
}
