use crate::common::{
    utils::{from_hash_map, invite_candidate_executors},
    ExecutionInfo,
};
use crate::delegator::sign::store_item::KeySliceInfo;
use prost::Message;
use std::{collections::HashMap, convert::TryFrom};
use store_item::{DelegatorSignStoreItem, StoreItemState};
use tea_actor_utility::{
    action,
    actor_nats::response_reply_with_subject,
    encode_protobuf,
    ipfs_p2p::{response_ipfs_p2p, send_message, P2pReplyType},
};
use wascc_actor::prelude::codec::messaging::BrokerMessage;
use wascc_actor::HandlerResult;

mod observers;
mod ra;
mod store_item;

pub use observers::{is_sign_tag, operation_after_verify_handler};

pub fn process_sign_with_key_slices_event(
    res: crate::actor_delegate_proto::SignTransactionResponse,
) -> anyhow::Result<()> {
    super::verifier::try_to_be_delegator(
        res.data_adhoc.delegator_tea_nonce_rsa_encryption.clone(),
        res.data_adhoc.delegator_tea_nonce_hash.clone(),
        move |nonce| {
            let res = res.clone();
            get_deployment_ids(res.multi_sig_account.clone(), move |deployment_ids| {
                let mut item = DelegatorSignStoreItem::try_from(res.clone())?;
                item.nonce = nonce.clone();
                // todo query p1 public key from layer1 by item.multi_sig_account, and verify item.p1_signature
                // todo query n,k,keyType and deployment ids from layer1 by item.multi_sig_account
                let n = 2;
                let k = 1;
                let task_type = "bitcoin_mainnet".to_string();

                let properties = ra::generate_pinner_ra_properties(&item.task_info.task_id);
                item.task_info.exec_info = ExecutionInfo { n, k, task_type };
                item.init_deployment_resources(&deployment_ids);
                item.state = StoreItemState::Initialized;
                DelegatorSignStoreItem::save(&item)?;

                invite_candidate_executors(
                    item.task_info.clone(),
                    move |task_info, peer_id| {
                        send_message(
                            &peer_id,
                            &task_info.task_id,
                            crate::p2p_proto::GeneralMsg {
                                msg: Some(
                                    crate::p2p_proto::general_msg::Msg::SignCandidateRequest(
                                        crate::p2p_proto::SignCandidateRequest {
                                            task_id: task_info.task_id.clone(),
                                            multi_sig_account: item.multi_sig_account.clone(),
                                        },
                                    ),
                                ),
                            },
                        )
                    },
                    move |task_info, _| {
                        for id in deployment_ids.iter() {
                            begin_find_pinners(id.to_string(), properties.clone())?;
                        }

                        let mut item = DelegatorSignStoreItem::get(&task_info.task_id)?;
                        item.state = StoreItemState::FindingDeployments;
                        DelegatorSignStoreItem::save(&item)?;
                        Ok(())
                    },
                )
            })?;
            Ok(())
        },
    )
}

#[cfg(feature = "dev")]
fn get_deployment_ids<F>(_multi_sig_account: Vec<u8>, mut callback: F) -> HandlerResult<()>
where
    F: FnMut(Vec<String>) -> anyhow::Result<()> + Send + Sync + 'static,
{
    let deployment_ids = super::dump_methods::get_deployment_ids()?;
    debug!("get mocked deployment_ids: {:?}", &deployment_ids);
    Ok(callback(deployment_ids)?)
}

#[cfg(not(feature = "dev"))]
fn get_deployment_ids<F>(multi_sig_account: Vec<u8>, mut callback: F) -> HandlerResult<()>
where
    F: FnMut(Vec<String>) -> anyhow::Result<()> + Send + Sync + 'static,
{
    let content = base64::encode(&encode_protobuf(
        crate::actor_delegate_proto::GetDeploymentIds { multi_sig_account },
    )?);
    action::call(
        "layer1.async.reply.get_deployment_ids",
        "actor.gluon.inbox",
        content.into(),
        move |msg| {
            let base64_decoded_msg_body = base64::decode(String::from_utf8(msg.body.clone())?)?;
            let get_deployments_res =
                crate::actor_delegate_proto::GetDeploymentIdsResponse::decode(
                    base64_decoded_msg_body.as_slice(),
                )?;
            debug!(
                "request for get_deployment_ids from layer1 got response: {:?}",
                &get_deployments_res
            );
            Ok(callback(get_deployments_res.asset_info.p2_deployment_ids)?)
        },
    )
}

fn begin_find_pinners(
    deployment_id: String,
    properties: HashMap<String, String>,
) -> anyhow::Result<()> {
    Ok(action::call_async_intercom(
        crate::PINNER_ACTOR_NAME,
        crate::MY_ACTOR_NAME,
        BrokerMessage {
            subject: "actor.pinner.intercom.find_pinners".into(),
            reply_to: "".into(),
            body: encode_protobuf(crate::actor_pinner_proto::FindPinnersRequest {
                send_to_actor: crate::MY_ACTOR_NAME.to_string(),
                deployment_id,
                properties: from_hash_map(properties),
                delay_seconds: 0,
                finding_mode: tea_codec::serialize(
                    tea_codec::ipfs_codec::FindingMode::AsMuchAsPossible,
                )?,
            })?,
        },
        move |msg| {
            debug!("begin_find_pinners got response: {:?}", msg);
            Ok(())
        },
    )
    .map_err(|e| anyhow::anyhow!("{}", e))?)
}

pub fn process_executor_sign_with_key_slices_request(
    req: crate::p2p_proto::TaskSignWithKeySlicesRequst,
    peer_id: &str,
    reply_to: &str,
) -> anyhow::Result<()> {
    let item = DelegatorSignStoreItem::get(&req.task_id)?;
    if item.executor.is_some() {
        return response_ipfs_p2p(
            reply_to,
            peer_id,
            &req.task_id,
            "executor already exists".into(),
            P2pReplyType::Rejected,
        );
    }

    ra::remote_attestation_executor(req, peer_id.to_string(), reply_to.to_string())?;
    response_reply_with_subject(
        "",
        reply_to,
        "has sent executor ra request".as_bytes().to_vec(),
    )
}

pub fn process_commit_sign_result_request(
    req: crate::p2p_proto::TaskCommitSignResultRequest,
    _peer_id: &str,
    reply_to: &str,
) -> anyhow::Result<()> {
    debug!("process_commit_sign_result_request req: {:?}", &req);
    let mut item = DelegatorSignStoreItem::get(&req.task_id)?;

    // todo verify signature using item.multi_sig_account

    // todo commit task_id and req.witness hash into layer1

    // todo send transaction to bitcoin network (or other network decided by item.task_info.exec_info.task_type)

    item.state = StoreItemState::CommitResult;
    DelegatorSignStoreItem::save(&item)?;

    info!("commit sign task successfully");

    response_reply_with_subject(
        "",
        reply_to,
        "commit signature successfully".as_bytes().to_vec(),
    )
}

pub fn process_pinner_key_slice_response(
    res: crate::p2p_proto::TaskSignGetPinnerKeySliceResponse,
    peer_id: &str,
    reply_to: &str,
) -> anyhow::Result<()> {
    let deployment_id: String = res.deployment_id.clone();
    let mut item = DelegatorSignStoreItem::get(&res.task_id)?;
    if item.has_found_key_slice(&deployment_id) {
        return response_ipfs_p2p(
            reply_to,
            peer_id,
            &res.task_id,
            &format!("pinner of {} already exists", &deployment_id),
            P2pReplyType::Rejected,
        );
    }

    item.insert_key_slice_info(
        &deployment_id,
        KeySliceInfo {
            peer_id: peer_id.to_string(),
            encrypted_key_slice: res.encrypted_key_slice.clone(),
        },
    )?;
    DelegatorSignStoreItem::save(&item)?;

    response_reply_with_subject(
        "",
        reply_to,
        format!("become pinner of {} successfully", &deployment_id)
            .as_bytes()
            .to_vec(),
    )?;
    try_send_to_executor(&mut item)
}

fn try_send_to_executor(item: &mut DelegatorSignStoreItem) -> anyhow::Result<()> {
    if !item.ready_send_to_executor() {
        return Ok(());
    }

    debug!("ready to send sign task to executor, task id: {}", &item.task_info.task_id);
    let encrypted_key_slices = item.get_encrypted_key_slices();
    let res = crate::p2p_proto::GeneralMsg {
        msg: Some(
            crate::p2p_proto::general_msg::Msg::TaskSignWithKeySlicesResponse(
                crate::p2p_proto::TaskSignWithKeySlicesResponse {
                    task_id: item.task_info.task_id.clone(),
                    adhoc_data: item.transaction_data.clone(),
                    p1_signature: item.p1_signature.clone(),
                    key_type: item.task_info.exec_info.task_type.clone(),
                    encrypted_key_slices,
                },
            ),
        ),
    };
    send_message(
        &item
            .executor
            .as_ref()
            .ok_or(anyhow::anyhow!(
                "{}:{} failed to get executor, task id is {}",
                file!(),
                line!(),
                &item.task_info.task_id
            ))?
            .peer_id,
        &item.task_info.task_id,
        res,
    )?;

    item.state = StoreItemState::SentToExecutor;
    DelegatorSignStoreItem::save(&item)
}
