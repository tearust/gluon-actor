use super::execution_info::ExecutionInfo;
use serde::export::TryFrom;
use tea_codec::error::TeaError;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInfo {
    pub task_id: String,
    pub exec_info: ExecutionInfo,
}

impl TryFrom<crate::actor_delegate_proto::KeyGenerationResponse> for TaskInfo {
    type Error = TeaError;

    fn try_from(
        value: crate::actor_delegate_proto::KeyGenerationResponse,
    ) -> Result<Self, Self::Error> {
        let item = TaskInfo {
            task_id: value.task_id,
            exec_info: ExecutionInfo {
                n: value.data_adhoc.n as u8,
                k: value.data_adhoc.k as u8,
                task_type: value.data_adhoc.key_type,
            },
        };
        if item.exec_info.n == 0 {
            return Err(TeaError::CommonError(format!(
                "{}:{} invalid value n",
                line!(),
                file!()
            )));
        }
        if item.exec_info.k == 0 || item.exec_info.k >= item.exec_info.n {
            return Err(TeaError::CommonError(format!(
                "{}:{} invalid value k",
                line!(),
                file!(),
            )));
        }

        Ok(item)
    }
}
