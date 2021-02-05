#![cfg(feature = "dev")]

use crate::{MY_ACTOR_NAME, PINNER_ACTOR_NAME};
use tea_actor_utility::{
    action,
    action::get_uuid,
    actor_crypto::{generate, sha256, sign},
    actor_nats::response_reply_to,
    actor_util::rsa_encrypt,
    encode_protobuf,
};
use wascc_actor::prelude::codec::messaging::BrokerMessage;
use wascc_actor::HandlerResult;

pub fn generate_sign_response_message(msg: &BrokerMessage) -> HandlerResult<()> {
    let message = String::from_utf8(msg.body.clone())?;
    let params: Vec<&str> = message.split(',').collect();
    if params.len() < 2 {
        error!("not enough params, params details: {:?}", &params);
    }

    let key_type = params[0].to_string();
    let multi_sig_account = params[1].as_bytes().to_vec();
    let reply_to = msg.reply_to.clone();
    debug!(
        "generate_key_gen_response_message with key type: {}, multi_sig_account: {}",
        &params[0], &params[1]
    );

    delegator_key_operation(
        reply_to,
        move |delegator_tea_nonce_hash, delegator_tea_nonce_rsa_encryption| {
            let transaction_data = "hello world!".as_bytes().to_vec();
            let (_, p1_priv_key) = generate(key_type.clone())?;
            let p1_signature = sign(key_type.clone(), p1_priv_key, transaction_data.clone())?;
            let res = crate::actor_delegate_proto::SignTransactionResponse {
                task_id: get_uuid(),
                data_adhoc: crate::actor_delegate_proto::SignTransactionData {
                    transaction_data,
                    delegator_tea_nonce_hash,
                    delegator_tea_nonce_rsa_encryption,
                },
                payment: crate::actor_delegate_proto::TaskPaymentDescription {},
                p1_signature,
                multi_sig_account: multi_sig_account.clone(),
            };
            Ok(base64::encode(encode_protobuf(res)?))
        },
    )?;
    Ok(())
}

pub fn generate_key_gen_response_message(msg: &BrokerMessage) -> HandlerResult<()> {
    let key_type = String::from_utf8(msg.body.clone())?;
    let reply_to = msg.reply_to.clone();
    debug!(
        "generate_key_gen_response_message with key type: {}",
        key_type
    );

    delegator_key_operation(
        reply_to,
        move |delegator_tea_nonce_hash, delegator_tea_nonce_rsa_encryption| {
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
            Ok(base64::encode(encode_protobuf(res)?))
        },
    )?;
    Ok(())
}

fn delegator_key_operation<F>(reply_to: String, mut operation: F) -> anyhow::Result<()>
where
    F: FnMut(Vec<u8>, Vec<u8>) -> anyhow::Result<String> + Clone + Sync + Send + 'static,
{
    action::call_async_intercom(
        PINNER_ACTOR_NAME,
        MY_ACTOR_NAME,
        BrokerMessage {
            subject: "actor.pinner.intercom.get_delegator_pub_key".into(),
            reply_to: "".into(),
            body: Vec::new(),
        },
        move |msg| {
            let pub_key: Option<Vec<u8>> = tea_codec::deserialize(msg.body.as_slice())?;
            let pub_key = pub_key.ok_or(anyhow::anyhow!("failed to get delegator key"))?;
            let nonce = wascc_actor::extras::default()
                .get_random(u32::MIN, u32::MAX)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            debug!("nonce is: {}", nonce);
            let nonce_bytes = nonce.to_le_bytes().to_vec();
            let delegator_tea_nonce_hash = sha256(nonce_bytes.clone())?;
            let delegator_tea_nonce_rsa_encryption = rsa_encrypt(pub_key, nonce_bytes)?;

            let content = operation(delegator_tea_nonce_hash, delegator_tea_nonce_rsa_encryption)?;
            response_reply_to(&reply_to, content.as_bytes().to_vec())?;
            Ok(())
        },
    )
}
