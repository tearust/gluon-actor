use crate::common::{ExecutionInfo, TaskInfo};
use crate::BINDING_NAME;
use serde::export::TryFrom;
use tea_actor_utility::actor_kvp;
use tea_actor_utility::actor_kvp::ShabbyLock;
use tea_codec::error::TeaError;

const PREFIX_EXECUTOR_TASK_STORE_ITEM: &'static str = "executor_task_store_item";

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub enum StoreItemState {
    Init,
    Requested,
    Responded,
    Executed,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutorStoreItem {
    pub task_info: TaskInfo,
    pub state: StoreItemState,
}

impl ExecutorStoreItem {
    pub fn get(task_id: &str) -> anyhow::Result<Self> {
        let _lock = ShabbyLock::lock(BINDING_NAME, task_id);
        actor_kvp::get::<ExecutorStoreItem>(BINDING_NAME, &get_task_store_item_key(task_id))?
            .ok_or(TeaError::CommonError(format!("can not find task {}", task_id)).into())
    }

    pub fn save(item: &ExecutorStoreItem) -> anyhow::Result<()> {
        let _lock = ShabbyLock::lock(BINDING_NAME, &item.task_info.task_id);
        actor_kvp::set_forever(
            BINDING_NAME,
            &get_task_store_item_key(&item.task_info.task_id),
            item,
        )?;
        Ok(())
    }
}

impl TryFrom<crate::actor_delegate_proto::KeyGenerationResponse> for ExecutorStoreItem {
    type Error = TeaError;

    fn try_from(
        value: crate::actor_delegate_proto::KeyGenerationResponse,
    ) -> Result<Self, Self::Error> {
        Ok(ExecutorStoreItem {
            task_info: TaskInfo::try_from(value)?,
            state: StoreItemState::Init,
        })
    }
}

impl TryFrom<crate::actor_delegate_proto::SignWithKeySlicesRequest> for ExecutorStoreItem {
    type Error = TeaError;

    fn try_from(
        value: crate::actor_delegate_proto::SignWithKeySlicesRequest,
    ) -> Result<Self, Self::Error> {
        Ok(ExecutorStoreItem {
            task_info: TaskInfo {
                task_id: value.task_id,
                exec_info: ExecutionInfo::default(),
            },
            state: StoreItemState::Init,
        })
    }
}

fn get_task_store_item_key(task_id: &str) -> String {
    format!("{}_{}", PREFIX_EXECUTOR_TASK_STORE_ITEM, task_id)
}
