use std::collections::HashMap;
use tea_actor_utility::action::{call_async_intercom, post_intercom};
use tea_actor_utility::actor_nats::response_reply_with_subject;
use tea_actor_utility::{action, encode_protobuf};
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

pub fn send_ra_request(
    peer_id: String,
    reply_to: String,
    properties: HashMap<String, String>,
) -> anyhow::Result<()> {
    Ok(call_async_intercom(
        crate::PINNER_ACTOR_NAME,
        BrokerMessage {
            subject: "actor.pinner.intercom.request_peer_approve_ra".into(),
            reply_to: "actor.task.inbox".into(),
            body: encode_protobuf(crate::actor_pinner_proto::PeerApproveRaRequest {
                peer_id,
                properties: from_hash_map(properties),
            })?,
        },
        move |msg| {
            debug!("remote_attestation_executor got response: {:?}", msg);
            Ok(())
        },
    )
    .map_err(|e| anyhow::anyhow!("{}", e))?)
}
