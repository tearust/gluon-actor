use std::collections::HashMap;
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

pub fn send_ra_request(peer_id: String, properties: HashMap<String, String>) -> anyhow::Result<()> {
    action::call_async_intercom(
        crate::PINNER_ACTOR_NAME,
        crate::MY_ACTOR_NAME,
        BrokerMessage {
            subject: "actor.pinner.intercom.request_peer_approve_ra".into(),
            reply_to: "".into(),
            body: encode_protobuf(crate::actor_pinner_proto::PeerApproveRaRequest {
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
