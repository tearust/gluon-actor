use crate::delegator::executor_info::ExecutorInfo;
use crate::delegator::key_gen::initial_pinner_info::InitialPinnerInfo;
use crate::delegator::key_gen::{ExecutorRequestConstructor, TaskCandidates};
use crate::{common::TaskInfo, BINDING_NAME};
use anyhow::anyhow;
use std::collections::HashMap;
use std::convert::TryFrom;
use tea_actor_utility::actor_kvp::{self, ShabbyLock};
use tea_codec::error::TeaError;

const PREFIX_DELEGATOR_TASK_KEY_GEN_STORE_ITEM: &'static str = "delegator_task_key_gen_store_item";

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub enum StoreItemState {
    Init,
    InvitedCandidates,
    RaBegun,
    RaCompleted,
    SentToExecutor,
    ReceivedExecutionResult,
    SentToInitialPinner,
    ReceivedAllPinnerResponse,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegatorKeyGenStoreItem {
    pub task_info: TaskInfo,
    pub state: StoreItemState,
    pub nonce: Vec<u8>,
    pub executor: Option<ExecutorInfo>,
    pub initial_pinners: Vec<InitialPinnerInfo>,
    pub p1_public_key: Vec<u8>,
    pub p2_public_key: Option<Vec<u8>>,
    pub multi_sig_account: Option<Vec<u8>>,
    pub initial_pinner_responses: HashMap<String, Option<String>>,
    candidate_executors: Vec<ExecutorInfo>,
    candidate_initial_pinners: Vec<InitialPinnerInfo>,
}

impl TaskCandidates for DelegatorKeyGenStoreItem {
    fn ready(&self) -> bool {
        // todo return true if timeout

        // todo only if count of candidates over than specified number

        // this is essential condition to construct an execution request
        return !self.candidate_executors.is_empty()
            && (self.candidate_executors.len() + self.candidate_initial_pinners.len())
                > (self.task_info.exec_info.n + 1) as usize;
    }

    fn insert_executor(&mut self, executor: ExecutorInfo) {
        self.candidate_executors.push(executor);
    }

    fn insert_initial_pinner(&mut self, pinner: InitialPinnerInfo) {
        self.candidate_initial_pinners.push(pinner);
    }

    fn elect(&mut self) -> anyhow::Result<()> {
        self.select_executor()?;
        self.select_initial_pinners()
    }
}

impl TryFrom<crate::actor_delegate_proto::KeyGenerationResponse> for DelegatorKeyGenStoreItem {
    type Error = TeaError;

    fn try_from(
        value: crate::actor_delegate_proto::KeyGenerationResponse,
    ) -> Result<Self, Self::Error> {
        let p1_public_key = value.p1_public_key.clone();
        Ok(DelegatorKeyGenStoreItem {
            task_info: TaskInfo::try_from(value)?,
            state: StoreItemState::Init,
            nonce: Vec::new(),
            executor: None,
            p1_public_key,
            p2_public_key: None,
            multi_sig_account: None,
            initial_pinners: Vec::new(),
            initial_pinner_responses: HashMap::new(),
            candidate_executors: Vec::new(),
            candidate_initial_pinners: Vec::new(),
        })
    }
}

impl ExecutorRequestConstructor for DelegatorKeyGenStoreItem {
    fn ready(&self) -> bool {
        return !self.executor.is_none()
            && self.initial_pinners.len() == self.task_info.exec_info.n as usize;
    }

    fn generate(&self) -> anyhow::Result<crate::p2p_proto::TaskExecutionRequest> {
        let mut initial_pinners: Vec<crate::p2p_proto::TaskExecutionInitialPinnerData> = Vec::new();
        for pinner in self.initial_pinners.iter() {
            initial_pinners.push(crate::p2p_proto::TaskExecutionInitialPinnerData {
                peer_id: pinner.peer_id.clone(),
                rsa_pub_key: pinner.rsa_pub_key.clone(),
            })
        }
        Ok(crate::p2p_proto::TaskExecutionRequest {
            task_id: self.task_info.task_id.clone(),
            initial_pinners,
            minimum_recovery_number: self.task_info.exec_info.k as u32,
            key_type: self.task_info.exec_info.task_type.clone(),
            p1_public_key: self.p1_public_key.clone(),
        })
    }
}

impl DelegatorKeyGenStoreItem {
    pub fn get(task_id: &str) -> anyhow::Result<Self> {
        let _lock = ShabbyLock::lock(BINDING_NAME, task_id);
        actor_kvp::get::<DelegatorKeyGenStoreItem>(BINDING_NAME, &get_task_store_item_key(task_id))?
            .ok_or(TeaError::CommonError(format!("can not find task {}", task_id)).into())
    }

    pub fn save(item: &DelegatorKeyGenStoreItem) -> anyhow::Result<()> {
        let _lock = ShabbyLock::lock(BINDING_NAME, &item.task_info.task_id);
        actor_kvp::set_forever(
            BINDING_NAME,
            &get_task_store_item_key(&item.task_info.task_id),
            item,
        )?;
        Ok(())
    }

    pub fn is_all_initial_pinners_ready(&self) -> bool {
        for (_, v) in self.initial_pinner_responses.iter() {
            if v.is_none() {
                return false;
            }
        }
        true
    }

    fn select_executor(&mut self) -> anyhow::Result<()> {
        // todo: min XOR value calculated by `block hash + task hash + candidate ephemeral id`
        //  of all candidates should be executor
        self.executor = Some(self.candidate_executors.pop().ok_or(anyhow!(
            "{}:{} candidate executor can not be empty",
            line!(),
            file!(),
        ))?);
        Ok(())
    }

    fn select_initial_pinners(&mut self) -> anyhow::Result<()> {
        while self.initial_pinners.len() < self.task_info.exec_info.n as usize
            || (self.candidate_initial_pinners.is_empty() && self.candidate_executors.is_empty())
        {
            if !self.candidate_initial_pinners.is_empty() {
                if let Some(p) = self.candidate_initial_pinners.pop() {
                    self.initial_pinners.push(p);
                }
                continue;
            }

            if let Some(e) = self.candidate_executors.pop() {
                self.initial_pinners.push(e.into());
            }
        }
        Ok(())
    }
}

fn get_task_store_item_key(task_id: &str) -> String {
    format!("{}_{}", PREFIX_DELEGATOR_TASK_KEY_GEN_STORE_ITEM, task_id)
}
