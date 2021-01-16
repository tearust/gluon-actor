use crate::{MY_ACTOR_NAME, PINNER_ACTOR_NAME};
use tea_actor_utility::{action, actor_crypto::sha256, actor_util::rsa_decrypt};
use wascc_actor::prelude::codec::messaging::BrokerMessage;
use wascc_actor::HandlerResult;

pub fn try_to_be_delegator<F>(
    nonce_encrypted: Vec<u8>,
    nonce_hash: Vec<u8>,
    mut callback: F,
) -> anyhow::Result<()>
where
    F: FnMut(Vec<u8>) -> HandlerResult<()> + Clone + Sync + Send + 'static,
{
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
            if let Some(key) = key {
                match rsa_decrypt(key, nonce_encrypted.clone()) {
                    Ok(nonce) => {
                        if !nonce_hash.eq(&sha256(nonce.clone())?) {
                            return Err("given hash and the hash calculated from decrypt nonce did not match".into());
                        }

                        return callback(nonce);
                    }
                    Err(e) => {
                        debug!("decrypt rsa error: {}, i'm not delegator", e);
                    }
                }
            }
            Ok(())
        },
    )
}
