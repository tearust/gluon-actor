use crate::delegator::executor_info::ExecutorInfo;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitialPinnerInfo {
    pub peer_id: String,
    pub rsa_pub_key: Vec<u8>,
}

impl From<ExecutorInfo> for InitialPinnerInfo {
    fn from(exe: ExecutorInfo) -> Self {
        InitialPinnerInfo {
            peer_id: exe.peer_id,
            rsa_pub_key: exe.rsa_pub_key,
        }
    }
}
