use crate::BINDING_NAME;
use std::collections::HashMap;

mod client_observer;

const PROPERTY_KEY_GEN_FLAG: &'static str = "task_delegator_key_gen_flag";

pub use client_observer::operation_after_verify_handler;

pub fn tag_for_key_gen(settings: &mut HashMap<String, String>) {
    settings.insert(PROPERTY_KEY_GEN_FLAG.into(), BINDING_NAME.into());
}

pub fn is_key_gen_tag(item: &crate::actor_pinner_proto::ChallangeStoreItem) -> bool {
    item.properties
        .iter()
        .find(|v| PROPERTY_KEY_GEN_FLAG.eq(&v.key))
        .unwrap_or(&crate::actor_pinner_proto::PropertyKeyPair::default())
        .value
        == BINDING_NAME
}
