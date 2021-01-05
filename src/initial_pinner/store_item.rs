use crate::common::TaskInfo;
use crate::BINDING_NAME;
use serde::export::TryFrom;
use tea_actor_utility::actor_kvp;
use tea_actor_utility::actor_kvp::ShabbyLock;
use tea_codec::error::TeaError;

const PREFIX_INITIAL_PINNER_TASK_STORE_ITEM: &'static str = "pinner_task_store_item";

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub enum StoreItemState {
    Init,
    Requested,
    Responded,
    Deployed,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitialPinnerStoreItem {
    pub task_info: TaskInfo,
    pub state: StoreItemState,
}

impl InitialPinnerStoreItem {
    pub fn get(task_id: &str) -> anyhow::Result<Self> {
        let _lock = ShabbyLock::lock(BINDING_NAME, task_id);
        actor_kvp::get::<InitialPinnerStoreItem>(BINDING_NAME, &get_task_store_item_key(task_id))?
            .ok_or(TeaError::CommonError(format!("can not find task {}", task_id)).into())
    }

    pub fn save(item: &InitialPinnerStoreItem) -> anyhow::Result<()> {
        let _lock = ShabbyLock::lock(BINDING_NAME, &item.task_info.task_id);
        actor_kvp::set_forever(
            BINDING_NAME,
            &get_task_store_item_key(&item.task_info.task_id),
            item,
        )?;
        Ok(())
    }
}

impl TryFrom<crate::actor_delegate_proto::KeyGenerationResponse> for InitialPinnerStoreItem {
    type Error = TeaError;

    fn try_from(
        value: crate::actor_delegate_proto::KeyGenerationResponse,
    ) -> Result<Self, Self::Error> {
        Ok(InitialPinnerStoreItem {
            task_info: TaskInfo::try_from(value)?,
            state: StoreItemState::Init,
        })
    }
}

fn get_task_store_item_key(task_id: &str) -> String {
    format!("{}_{}", PREFIX_INITIAL_PINNER_TASK_STORE_ITEM, task_id)
}
