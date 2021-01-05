use std::collections::HashMap;
use tea_actor_utility::actor_nats::response_reply_with_subject;
use tea_actor_utility::{action, encode_protobuf};

pub fn from_hash_map(
    items: HashMap<String, String>,
) -> Vec<crate::actor_pinner_proto::PropertyKeyPair> {
    let mut properties: Vec<crate::actor_pinner_proto::PropertyKeyPair> = Vec::new();
    for (key, value) in items {
        properties.push(crate::actor_pinner_proto::PropertyKeyPair { key, value });
    }
    properties
}

pub fn send_ra_request(
    peer_id: String,
    reply_to: String,
    properties: HashMap<String, String>,
    response_msg: String,
) -> anyhow::Result<()> {
    Ok(action::call(
        "actor.pinner.inbox.request_peer_approve_ra",
        "actor.task.inbox",
        encode_protobuf(crate::actor_pinner_proto::PeerApproveRaRequest {
            peer_id,
            properties: from_hash_map(properties),
        })?,
        move |msg| {
            debug!("remote_attestation_executor got response: {:?}", msg);
            Ok(response_reply_with_subject(
                "",
                &reply_to,
                response_msg.as_bytes().to_vec(),
            )?)
        },
    )
    .map_err(|e| anyhow::anyhow!("{}", e))?)
}
