use std::convert::TryFrom;
use store_item::{DelegatorSignStoreItem, StoreItemState};
use tea_actor_utility::actor_nats::response_reply_with_subject;

mod observers;
mod ra;
mod store_item;

use crate::common::utils::from_hash_map;
use crate::common::ExecutionInfo;
use crate::delegator::sign::store_item::KeySliceInfo;
pub use observers::{is_sign_tag, operation_after_verify_handler};
use std::collections::HashMap;
use tea_actor_utility::ipfs_p2p::{response_ipfs_p2p, send_message, P2pReplyType};
use tea_actor_utility::{action, encode_protobuf};

pub fn process_sign_with_key_slices_event(
    req: crate::actor_delegate_proto::SignWithKeySlicesRequest,
) -> anyhow::Result<()> {
    let mut item = DelegatorSignStoreItem::try_from(req)?;

    // todo query p1 public key from layer1 by item.multi_sig_account, and verify item.p1_signature

    DelegatorSignStoreItem::save(&item)?;

    let properties = ra::generate_pinner_ra_properties(&item.task_info.task_id);

    // todo query n,k,keyType and deployment ids from layer1 by item.multi_sig_account
    let n = u8::default();
    let k = u8::default();
    let task_type = String::default();
    let deployment_ids: Vec<String> = Vec::new();

    item.task_info.exec_info = ExecutionInfo { n, k, task_type };
    item.init_deployment_resources(&deployment_ids);
    for id in deployment_ids {
        begin_find_pinners(id, properties.clone())?;
    }

    item.state = StoreItemState::FindingDeployments;
    DelegatorSignStoreItem::save(&item)?;
    Ok(())
}

fn begin_find_pinners(
    deployment_id: String,
    properties: HashMap<String, String>,
) -> anyhow::Result<()> {
    Ok(action::call(
        "actor.pinner.inbox.find_pinners",
        "actor.task.inbox",
        encode_protobuf(crate::actor_pinner_proto::FindPinnersRequest {
            deployment_id,
            properties: from_hash_map(properties),
            delay_seconds: 0,
            finding_mode: tea_codec::serialize(
                tea_codec::ipfs_codec::FindingMode::AsMuchAsPossible,
            )?,
        })?,
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
    let mut item = DelegatorSignStoreItem::get(&req.task_id)?;

    // todo verify signature using item.multi_sig_account

    // todo commit task_id and req.witness hash into layer1

    // todo send transaction to bitcoin network (or other network decided by item.task_info.exec_info.task_type)

    item.state = StoreItemState::CommitResult;
    DelegatorSignStoreItem::save(&item)?;

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

    let encrypted_key_slices = item.get_encrypted_key_slices();
    let res = crate::p2p_proto::GeneralMsg {
        msg: Some(
            crate::p2p_proto::general_msg::Msg::TaskSignWithKeySlicesResponse(
                crate::p2p_proto::TaskSignWithKeySlicesResponse {
                    task_id: item.task_info.task_id.clone(),
                    adhoc_data: item.adhoc_data.clone(),
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
