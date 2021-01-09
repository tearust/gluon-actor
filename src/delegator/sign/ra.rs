use crate::common::utils::send_ra_request;
use crate::delegator::sign::observers::tag_for_sign;
use std::collections::HashMap;
use tea_actor_utility::actor_nats::response_reply_with_subject;

pub const PROPERTY_TASK_ID: &'static str = "task_id";
pub const PROPERTY_RSA_PUB_KEY: &'static str = "rsa_pub_key";
const PROPERTY_DELEGATOR_RA_TARGET_ROLE: &'static str = "delegator_sign_ra_target_role";
const VALUE_RA_TARGET_EXECUTOR: &'static str = "executor";
const VALUE_RA_TARGET_PINNER: &'static str = "pinner";

pub fn remote_attestation_executor(
    request: crate::p2p_proto::TaskSignWithKeySlicesRequst,
    peer_id: String,
    reply_to: String,
) -> anyhow::Result<()> {
    let mut properties: HashMap<String, String> = HashMap::new();
    properties.insert(PROPERTY_TASK_ID.into(), request.task_id.clone());
    properties.insert(
        PROPERTY_RSA_PUB_KEY.into(),
        base64::encode(&request.rsa_pub_key),
    );
    properties.insert(
        PROPERTY_DELEGATOR_RA_TARGET_ROLE.into(),
        VALUE_RA_TARGET_EXECUTOR.into(),
    );
    tag_for_sign(&mut properties);
    response_reply_with_subject(
        "",
        &reply_to,
        "sign to ra executor sent".as_bytes().to_vec(),
    )?;
    send_ra_request(peer_id, properties)
}

pub fn generate_pinner_ra_properties(task_id: &str) -> HashMap<String, String> {
    let mut properties: HashMap<String, String> = HashMap::new();
    properties.insert(PROPERTY_TASK_ID.into(), task_id.into());
    properties.insert(
        PROPERTY_DELEGATOR_RA_TARGET_ROLE.into(),
        VALUE_RA_TARGET_PINNER.into(),
    );
    tag_for_sign(&mut properties);
    properties
}

pub fn is_executor_ra_response(item: &crate::actor_pinner_proto::ChallangeStoreItem) -> bool {
    item.properties
        .iter()
        .find(|v| PROPERTY_DELEGATOR_RA_TARGET_ROLE.eq(&v.key))
        .unwrap_or(&crate::actor_pinner_proto::PropertyKeyPair::default())
        .value
        == VALUE_RA_TARGET_EXECUTOR
}

pub fn is_pinner_ra_response(item: &crate::actor_pinner_proto::ChallangeStoreItem) -> bool {
    item.properties
        .iter()
        .find(|v| PROPERTY_DELEGATOR_RA_TARGET_ROLE.eq(&v.key))
        .unwrap_or(&crate::actor_pinner_proto::PropertyKeyPair::default())
        .value
        == VALUE_RA_TARGET_PINNER
}
