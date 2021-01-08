use crate::common::utils::send_ra_request;
use crate::delegator::key_gen::observers::tag_for_key_gen;
use std::collections::HashMap;

pub const PROPERTY_TASK_ID: &'static str = "task_id";
pub const PROPERTY_RSA_PUB_KEY: &'static str = "rsa_pub_key";
const PROPERTY_DELEGATOR_RA_TARGET_ROLE: &'static str = "delegator_ra_target_role";
const VALUE_RA_TARGET_EXECUTOR: &'static str = "executor";
const VALUE_RA_TARGET_INITIAL_PINNER: &'static str = "initial_pinner";

pub fn remote_attestation_executor(
    request: crate::p2p_proto::TaskKeyGenerationApplyRequst,
    peer_id: String,
    reply_to: String,
) -> anyhow::Result<()> {
    debug!("remote_attestation_executor with request: {:?}", &request);
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
    tag_for_key_gen(&mut properties);
    send_ra_request(peer_id, reply_to, properties, "executor ra sent".into())
}

pub fn remote_attestation_initial_pinner(
    request: crate::p2p_proto::TaskKeyGenerationApplyRequst,
    peer_id: String,
    reply_to: String,
) -> anyhow::Result<()> {
    debug!("remote_attestation_executor with request: {:?}", &request);
    let mut properties: HashMap<String, String> = HashMap::new();
    properties.insert(PROPERTY_TASK_ID.into(), request.task_id.clone());
    properties.insert(
        PROPERTY_RSA_PUB_KEY.into(),
        base64::encode(request.rsa_pub_key),
    );
    properties.insert(
        PROPERTY_DELEGATOR_RA_TARGET_ROLE.into(),
        VALUE_RA_TARGET_INITIAL_PINNER.into(),
    );
    tag_for_key_gen(&mut properties);
    send_ra_request(
        peer_id,
        reply_to,
        properties,
        "initial pinner ra sent".into(),
    )
}

pub fn is_executor_ra_response(item: &crate::actor_pinner_proto::ChallangeStoreItem) -> bool {
    item.properties
        .iter()
        .find(|v| PROPERTY_DELEGATOR_RA_TARGET_ROLE.eq(&v.key))
        .unwrap_or(&crate::actor_pinner_proto::PropertyKeyPair::default())
        .value
        == VALUE_RA_TARGET_EXECUTOR
}

pub fn is_initial_pinner_ra_response(item: &crate::actor_pinner_proto::ChallangeStoreItem) -> bool {
    item.properties
        .iter()
        .find(|v| PROPERTY_DELEGATOR_RA_TARGET_ROLE.eq(&v.key))
        .unwrap_or(&crate::actor_pinner_proto::PropertyKeyPair::default())
        .value
        == VALUE_RA_TARGET_INITIAL_PINNER
}
