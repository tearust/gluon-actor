use crate::common::TaskInfo;
use prost::Message;
use std::collections::HashMap;
use tea_actor_utility::{action, encode_protobuf, layer1::lookup_node_profile_by_tea_id};
use wascc_actor::prelude::codec::messaging::BrokerMessage;

pub fn from_hash_map(
    items: HashMap<String, String>,
) -> Vec<crate::actor_pinner_proto::PropertyKeyPair> {
    let mut properties: Vec<crate::actor_pinner_proto::PropertyKeyPair> = Vec::new();
    for (key, value) in items {
        properties.push(crate::actor_pinner_proto::PropertyKeyPair { key, value });
    }
    properties
}

pub fn send_ra_request(peer_id: String, properties: HashMap<String, String>) -> anyhow::Result<()> {
    action::call_async_intercom(
        crate::PINNER_ACTOR_NAME,
        crate::MY_ACTOR_NAME,
        BrokerMessage {
            subject: "actor.pinner.intercom.request_peer_approve_ra".into(),
            reply_to: "".into(),
            body: encode_protobuf(crate::actor_pinner_proto::PeerApproveRaRequest {
                send_to_actor: crate::MY_ACTOR_NAME.to_string(),
                peer_id,
                properties: from_hash_map(properties),
            })?,
        },
        move |msg| {
            debug!("send_ra_request got response msg: {:?}", msg);
            Ok(())
        },
    )
}

pub fn invite_candidate_executors<F, O>(
    task_info: TaskInfo,
    candidate_op: O,
    mut callback: F,
) -> anyhow::Result<()>
where
    F: FnMut(TaskInfo, Vec<String>) -> anyhow::Result<()> + Clone + Sync + Send + 'static,
    O: FnMut(TaskInfo, String) -> anyhow::Result<()> + Clone + Sync + Send + 'static,
{
    let request = crate::actor_delegate_proto::GetDelegatesRequest {
        start: 0,
        limit: task_info.exec_info.n as u32,
    };

    let content = base64::encode(&encode_protobuf(request)?);
    debug!("get_delegates request content: {}", &content);
    action::call(
        "layer1.async.reply.get_delegates",
        "actor.gluon.inbox",
        content.into(),
        move |msg| {
            let base64_decoded_msg_body = base64::decode(String::from_utf8(msg.body.clone())?)?;
            let get_delegates_res = crate::actor_delegate_proto::GetDelegatesResponse::decode(
                base64_decoded_msg_body.as_slice(),
            )?;
            let candidates_tea_ids: Vec<Vec<u8>> = get_delegates_res
                .delegates
                .iter()
                .map(|v| v.tea_id.clone())
                .collect();

            for tea_id in candidates_tea_ids {
                let task_info = task_info.clone();
                let mut candidate_op = candidate_op.clone();
                lookup_node_profile_by_tea_id(&tea_id, "actor.gluon.inbox", move |profile| {
                    Ok(candidate_op(task_info.clone(), profile.peer_id.clone())?)
                })
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            }

            let task_info = task_info.clone();
            let peer_ids: Vec<String> = get_delegates_res
                .delegates
                .iter()
                .map(|v| v.peer_id.clone())
                .collect();
            debug!("get_delegates got response with peer_ids: {:?}", &peer_ids);
            Ok(callback(task_info, peer_ids)?)
        },
    )
    .map_err(|e| anyhow::anyhow!("{}", e))
}
