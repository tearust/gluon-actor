#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutorInfo {
    pub peer_id: String,
    pub rsa_pub_key: Vec<u8>,
}
