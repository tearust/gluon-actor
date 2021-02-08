use crate::common::{ExecutionInfo, TaskInfo};
use crate::delegator::executor_info::ExecutorInfo;
use crate::BINDING_NAME;
use std::collections::HashMap;
use std::convert::TryFrom;
use tea_actor_utility::actor_kvp;
use tea_actor_utility::actor_kvp::ShabbyLock;
use tea_codec::error::TeaError;

const PREFIX_DELEGATOR_TASK_SIGN_STORE_ITEM: &'static str = "delegator_task_sign_store_item";

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub enum StoreItemState {
    Init,
    Initialized,
    FindingDeployments,
    SentToExecutor,
    CommitResult,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeySliceInfo {
    pub peer_id: String,
    pub encrypted_key_slice: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegatorSignStoreItem {
    pub task_info: TaskInfo,
    pub state: StoreItemState,
    pub executor: Option<ExecutorInfo>,
    pub multi_sig_account: Vec<u8>,
    pub p1_signature: Vec<u8>,
    pub transaction_data: Vec<u8>,
    pub nonce: Vec<u8>,
    key_slices: HashMap<String, Option<KeySliceInfo>>,
    deployment_candidates: HashMap<String, Vec<String>>,
}

impl TryFrom<crate::actor_delegate_proto::SignTransactionResponse> for DelegatorSignStoreItem {
    type Error = TeaError;

    fn try_from(
        value: crate::actor_delegate_proto::SignTransactionResponse,
    ) -> Result<Self, Self::Error> {
        Ok(DelegatorSignStoreItem {
            task_info: TaskInfo {
                task_id: base64::encode(&value.task_id),
                exec_info: ExecutionInfo {
                    n: 0,
                    k: 0,
                    task_type: "".to_string(),
                },
            },
            nonce: Vec::new(),
            state: StoreItemState::Init,
            executor: None,
            multi_sig_account: value.multi_sig_account,
            p1_signature: value.p1_signature,
            transaction_data: value.data_adhoc.transaction_data,
            key_slices: HashMap::new(),
            deployment_candidates: HashMap::new(),
        })
    }
}

impl DelegatorSignStoreItem {
    pub fn get(task_id: &str) -> anyhow::Result<Self> {
        let _lock = ShabbyLock::lock(BINDING_NAME, task_id);
        actor_kvp::get::<DelegatorSignStoreItem>(BINDING_NAME, &get_task_store_item_key(task_id))?
            .ok_or(TeaError::CommonError(format!("can not find task {}", task_id)).into())
    }

    pub fn save(item: &DelegatorSignStoreItem) -> anyhow::Result<()> {
        let _lock = ShabbyLock::lock(BINDING_NAME, &item.task_info.task_id);
        actor_kvp::set_forever(
            BINDING_NAME,
            &get_task_store_item_key(&item.task_info.task_id),
            item,
        )?;
        Ok(())
    }

    pub fn init_deployment_resources(&mut self, deployment_ids: &Vec<String>) {
        for id in deployment_ids {
            self.key_slices.insert(id.clone(), None);
            self.deployment_candidates.insert(id.clone(), Vec::new());
        }
    }

    pub fn has_found_key_slice(&self, deployment_id: &str) -> bool {
        match self.key_slices.get(deployment_id) {
            Some(item) => item.is_some(),
            None => false,
        }
    }

    pub fn insert_deployment(&mut self, deployment_id: &str, peer_id: &str) -> anyhow::Result<()> {
        self.deployment_candidates
            .get_mut(deployment_id)
            .ok_or(anyhow::anyhow!(
                "{}:{} deployment_id {} not exists",
                line!(),
                file!(),
                deployment_id
            ))?
            .push(peer_id.to_string());
        Ok(())
    }

    pub fn insert_key_slice_info(
        &mut self,
        deployment_id: &str,
        info: KeySliceInfo,
    ) -> anyhow::Result<()> {
        *self
            .key_slices
            .get_mut(deployment_id)
            .ok_or(anyhow::anyhow!(
                "{}:{} deployment_id {} not exists",
                line!(),
                file!(),
                deployment_id
            ))? = Some(info);
        Ok(())
    }

    pub fn ready_send_to_executor(&self) -> bool {
        self.key_slices
            .iter()
            .filter(|(_, value)| value.is_some())
            .collect::<HashMap<&String, &Option<KeySliceInfo>>>()
            .len()
            >= self.task_info.exec_info.k as usize
    }

    pub fn pop_all_candidates(&mut self) -> HashMap<String, Vec<String>> {
        self.deployment_candidates.drain().collect()
    }

    pub fn get_encrypted_key_slices(&self) -> Vec<Vec<u8>> {
        let mut rtn: Vec<Vec<u8>> = Vec::new();
        for (_, v) in self.key_slices.iter() {
            if let Some(info) = v {
                rtn.push(info.encrypted_key_slice.clone());
            }
        }
        rtn
    }
}

fn get_task_store_item_key(task_id: &str) -> String {
    format!("{}_{}", PREFIX_DELEGATOR_TASK_SIGN_STORE_ITEM, task_id)
}
