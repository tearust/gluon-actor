#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionInfo {
    pub n: u8,
    pub k: u8,
    pub task_type: String,
}

impl Default for ExecutionInfo {
    fn default() -> Self {
        ExecutionInfo {
            n: 0,
            k: 0,
            task_type: "".into(),
        }
    }
}
