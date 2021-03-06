use crate::common::{send_key_candidate_request, utils::invite_candidate_executors};
use crate::delegator::executor_info::ExecutorInfo;
use crate::delegator::key_gen::initial_pinner_info::InitialPinnerInfo;
use std::convert::{TryFrom, TryInto};
use store_item::{DelegatorKeyGenStoreItem, StoreItemState};
use tea_actor_utility::{
    action,
    actor_nats::response_reply_with_subject,
    encode_protobuf,
    ipfs_p2p::{log_and_response, log_and_response_with_error, send_message, P2pReplyType},
};

mod candidates;
mod initial_pinner_info;
mod observers;
mod ra;
mod store_item;

pub use observers::{is_key_gen_tag, operation_after_verify_handler};
pub use ra::{remote_attestation_executor, remote_attestation_initial_pinner};
use tea_actor_utility::ipfs_p2p::close_p2p;
use wascc_actor::HandlerResult;

pub trait TaskCandidates {
    fn ready(&self) -> bool;
    fn insert_executor(&mut self, executor: ExecutorInfo);
    fn insert_initial_pinner(&mut self, pinner: InitialPinnerInfo);
    fn elect(&mut self) -> anyhow::Result<()>;
}

pub trait ExecutorRequestConstructor {
    fn ready(&self) -> bool;
    fn generate(&self) -> anyhow::Result<crate::p2p_proto::TaskExecutionRequest>;
}

pub fn process_key_generation_event(
    res: crate::actor_delegate_proto::KeyGenerationResponse,
) -> anyhow::Result<()> {
    super::verifier::try_to_be_delegator(
        res.data_adhoc.delegator_tea_nonce_rsa_encryption.clone(),
        res.data_adhoc.delegator_tea_nonce_hash.clone(),
        move |nonce| {
            trace!("i'm delegator, continue to invite executors and initial pinners");
            let mut store_item = DelegatorKeyGenStoreItem::try_from(res.clone())?;
            store_item.nonce = nonce;
            DelegatorKeyGenStoreItem::save(&store_item)?;

            invite_candidate_executors(
                store_item.task_info.clone(),
                |task_info, peer_id| {
                    debug!("begin to invite executor delegate {}", &peer_id);
                    send_key_candidate_request(&peer_id, task_info, true)?;
                    Ok(())
                },
                |task_info, peer_ids| {
                    Ok(candidates::invite_candidate_initial_pinners(
                        task_info, peer_ids,
                    )?)
                },
            )?;
            store_item.state = StoreItemState::InvitedCandidates;
            DelegatorKeyGenStoreItem::save(&store_item)?;
            Ok(())
        },
    )
}

fn try_send_to_executor(item: &mut DelegatorKeyGenStoreItem) -> anyhow::Result<()> {
    if (item as &mut dyn TaskCandidates).ready() {
        item.elect()?;
    } else {
        debug!("continue to wait more candidates...");
        return Ok(());
    }

    if !(item as &mut dyn ExecutorRequestConstructor).ready() {
        return Err(anyhow::anyhow!(
            "executor request {} can not be construct because did not elect properly",
            &item.task_info.task_id
        ));
    }

    info!(
        "collect enough candidates, begin to send request to executor: {}",
        &item.executor.as_ref().unwrap().peer_id
    );
    let req = item.generate()?;
    send_message(
        &item.executor.as_ref().unwrap().peer_id,
        &item.task_info.task_id,
        crate::p2p_proto::GeneralMsg {
            msg: Some(crate::p2p_proto::general_msg::Msg::TaskExecutionRequest(
                req,
            )),
        },
    )?;

    item.state = StoreItemState::SentToExecutor;
    DelegatorKeyGenStoreItem::save(item)
}

pub fn process_task_execution_response(
    res: crate::p2p_proto::TaskExecutionResponse,
    peer_id: &str,
    reply_to: &str,
) -> anyhow::Result<()> {
    debug!(
        "process_task_execution_response from {} with response {:?}",
        peer_id, &res
    );
    delegator_store_item_handler(&res.task_id, peer_id, reply_to, |item| {
        let mut item = item;
        item.p2_public_key = Some(res.p2_public_key.clone());
        item.multi_sig_account = Some(res.multi_sig_account.clone());
        item.state = StoreItemState::ReceivedExecutionResult;
        DelegatorKeyGenStoreItem::save(&item)?;

        for pinner_data in res.initial_pinners.iter() {
            share_slices_to_initial_pinner(
                &item.task_info.task_id,
                pinner_data,
                &res.p2_public_key,
                &res.multi_sig_account,
            )?;
            item.initial_pinner_responses
                .insert(pinner_data.peer_id.clone(), None);
        }
        item.state = StoreItemState::SentToInitialPinner;
        DelegatorKeyGenStoreItem::save(&item)?;

        response_reply_with_subject("", reply_to, "received task response".as_bytes().to_vec())
    })
}

pub fn process_task_pinner_key_slice_response(
    res: crate::p2p_proto::TaskPinnerKeySliceResponse,
    peer_id: &str,
    reply_to: &str,
) -> anyhow::Result<()> {
    debug!(
        "process_task_pinner_key_slice_response from {} with response: {:?}",
        peer_id, &res
    );
    delegator_store_item_handler(&res.task_id, peer_id, reply_to, |item| {
        let mut item = item;
        match item.initial_pinner_responses.get_mut(peer_id) {
            Some(response_value) => {
                *response_value = Some(res.deployment_id.clone());
            }
            None => {
                return log_and_response(
                    reply_to,
                    peer_id,
                    &res.task_id,
                    &format!(
                        "failed to get initial pinner's (peer id is {}) info with task_id of {}",
                        peer_id, &res.task_id
                    ),
                    P2pReplyType::Rejected,
                )
                .map_err(|e| anyhow::anyhow!("{}", e))
            }
        }

        if item.is_all_initial_pinners_ready() {
            debug!("all initial pinners ready, begin to update key generation result");
            item.state = StoreItemState::ReceivedAllPinnerResponse;
            DelegatorKeyGenStoreItem::save(&item)?;

            let result: crate::actor_delegate_proto::UpdateKeyGenerationResult =
                item.clone().try_into()?;
            let task_id = item.task_info.task_id.clone();
            action::call(
                "layer1.async.reply.update_generate_key_result",
                "actor.gluon.inbox",
                base64::encode(&encode_protobuf(result)?).into(),
                move |msg| {
                    debug!("update_generate_key_result got response: {:?}", msg);
                    close_p2p_connections(&task_id)
                },
            )
            .map_err(|e| anyhow::anyhow!("{}", e))
        } else {
            DelegatorKeyGenStoreItem::save(&item)?;
            response_reply_with_subject(
                "",
                reply_to,
                "received initial pinner key slice response"
                    .as_bytes()
                    .to_vec(),
            )
        }
    })
}

fn close_p2p_connections(task_id: &str) -> HandlerResult<()> {
    let item = DelegatorKeyGenStoreItem::get(task_id)?;
    for pinner in item.initial_pinners.iter() {
        close_p2p(&pinner.peer_id)?;
    }
    if let Some(executor) = item.executor {
        close_p2p(&executor.peer_id)?;
    }
    Ok(())
}

fn delegator_store_item_handler<T>(
    task_id: &str,
    peer_id: &str,
    reply_to: &str,
    action: T,
) -> anyhow::Result<()>
where
    T: Fn(&mut DelegatorKeyGenStoreItem) -> anyhow::Result<()>,
{
    match DelegatorKeyGenStoreItem::get(task_id) {
        Ok(ref mut item) => action(item),
        Err(e) => log_and_response_with_error(
            reply_to,
            peer_id,
            task_id,
            &format!(
                "failed to get DelegatorStoreItem with task_id {}, details: {}",
                task_id, e
            ),
        ),
    }
}

fn share_slices_to_initial_pinner(
    task_id: &str,
    data: &crate::p2p_proto::TaskResultInitialPinnerData,
    pub_key: &[u8],
    multi_sig_account: &[u8],
) -> anyhow::Result<()> {
    send_message(
        &data.peer_id,
        task_id,
        crate::p2p_proto::GeneralMsg {
            msg: Some(
                crate::p2p_proto::general_msg::Msg::TaskPinnerKeySliceRequest(
                    crate::p2p_proto::TaskPinnerKeySliceRequest {
                        task_id: task_id.to_string(),
                        public_key: pub_key.to_vec(),
                        encrypted_key_slice: data.encrypted_key_slice.clone(),
                        multi_sig_account: multi_sig_account.to_vec(),
                    },
                ),
            ),
        },
    )
}
