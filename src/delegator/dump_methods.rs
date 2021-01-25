#![cfg(feature = "dev")]

use crate::{MY_ACTOR_NAME, PINNER_ACTOR_NAME};
use tea_actor_utility::{
    action,
    action::get_uuid,
    actor_crypto::{generate, sha256},
    actor_nats::response_reply_to,
    actor_util::rsa_encrypt,
    encode_protobuf,
};
use wascc_actor::prelude::codec::messaging::BrokerMessage;
use wascc_actor::HandlerResult;

pub fn generate_key_gen_response_message(msg: &BrokerMessage) -> HandlerResult<()> {
    trace!("generate_key_gen_response_message msg: {:?}", msg);
    let key_type = String::from_utf8(msg.body.clone())?;
    let reply_to = msg.reply_to.clone();

    action::call_async_intercom(
        PINNER_ACTOR_NAME,
        MY_ACTOR_NAME,
        BrokerMessage {
            subject: "actor.pinner.intercom.get_delegator_key".into(),
            reply_to: "".into(),
            body: Vec::new(),
        },
        move |msg| {
            let key: Option<Vec<u8>> = tea_codec::deserialize(msg.body.as_slice())?;
            let key = key.ok_or(anyhow::anyhow!("failed to get delegator key"))?;
            let nonce = wascc_actor::extras::default()
                .get_random(u32::MIN, u32::MAX)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            debug!("nonce is: {}", nonce);
            let nonce_bytes = nonce.to_le_bytes().to_vec();
            let delegator_tea_nonce_hash = sha256(nonce_bytes.clone())?;
            let delegator_tea_nonce_rsa_encryption = rsa_encrypt(key, nonce_bytes)?;

            let (p1_public_key, _) = generate(key_type.clone())?;
            let res = crate::actor_delegate_proto::KeyGenerationResponse {
                task_id: get_uuid(),
                data_adhoc: crate::actor_delegate_proto::KeyGenerationData {
                    n: 2,
                    k: 1,
                    key_type: key_type.clone(),
                    delegator_tea_nonce_hash,
                    delegator_tea_nonce_rsa_encryption,
                },
                payment: crate::actor_delegate_proto::TaskPaymentDescription {},
                p1_public_key,
            };
            let encoded_res = base64::encode(encode_protobuf(res)?);
            response_reply_to(&reply_to, encoded_res.as_bytes().to_vec())?;
            Ok(())
        },
    )?;
    Ok(())
}
