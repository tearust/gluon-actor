use super::execution_info::ExecutionInfo;
use serde::export::TryFrom;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInfo {
    pub task_id: String,
    pub exec_info: ExecutionInfo,
}

impl TryFrom<crate::p2p_proto::KeyGenerationCandidateRequest> for TaskInfo {
    type Error = anyhow::Error;

    fn try_from(
        value: crate::p2p_proto::KeyGenerationCandidateRequest,
    ) -> Result<Self, Self::Error> {
        let info = TaskInfo {
            task_id: value.task_id,
            exec_info: ExecutionInfo {
                n: value.n as u8,
                k: value.k as u8,
                task_type: value.key_type,
            },
        };
        validate_task_info(info)
    }
}

impl From<crate::p2p_proto::SignCandidateRequest> for TaskInfo {
    fn from(value: crate::p2p_proto::SignCandidateRequest) -> Self {
        TaskInfo {
            task_id: value.task_id,
            exec_info: ExecutionInfo {
                n: value.n as u8,
                k: value.k as u8,
                task_type: value.task_type,
            },
        }
    }
}

impl TryFrom<crate::actor_delegate_proto::KeyGenerationResponse> for TaskInfo {
    type Error = anyhow::Error;

    fn try_from(
        value: crate::actor_delegate_proto::KeyGenerationResponse,
    ) -> Result<Self, Self::Error> {
        let info = TaskInfo {
            task_id: base64::encode(&value.task_id),
            exec_info: ExecutionInfo {
                n: value.data_adhoc.n as u8,
                k: value.data_adhoc.k as u8,
                task_type: value.data_adhoc.key_type,
            },
        };
        validate_task_info(info)
    }
}

fn validate_task_info(info: TaskInfo) -> anyhow::Result<TaskInfo> {
    if info.exec_info.n == 0 {
        return Err(anyhow::anyhow!("{}:{} invalid value n", line!(), file!()));
    }
    if info.exec_info.k == 0 || info.exec_info.k >= info.exec_info.n {
        return Err(anyhow::anyhow!("{}:{} invalid value k", line!(), file!(),));
    }

    Ok(info)
}
