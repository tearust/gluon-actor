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

            candidates::invite_candidate_executors(&store_item, |task_info, peer_ids| {
                Ok(candidates::invite_candidate_initial_pinners(
                    task_info, peer_ids,
                )?)
            })?;
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
    info!("collect enough candidates, begin to send request to executor");

    if !(item as &mut dyn ExecutorRequestConstructor).ready() {
        return Err(anyhow::anyhow!(
            "executor request {} can not be construct because did not elect properly",
            &item.task_info.task_id
        ));
    }

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
            item.state = StoreItemState::ReceivedAllPinnerResponse;
            DelegatorKeyGenStoreItem::save(&item)?;

            let result: crate::actor_delegate_proto::UpdateKeyGenerationResult =
                item.clone().try_into()?;
            let reply_to = reply_to.to_string();
            action::call(
                "layer1.async.reply.update_generate_key_result",
                "actor.gluon.inbox",
                base64::encode(&encode_protobuf(result)?).into(),
                move |msg| {
                    debug!("update_generate_key_result go response: {:?}", msg);
                    response_reply_with_subject(
                        "",
                        &reply_to,
                        "received initial pinner key slice response"
                            .as_bytes()
                            .to_vec(),
                    )?;
                    Ok(())
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
                    },
                ),
            ),
        },
    )
}
