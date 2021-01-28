use crate::delegator::process_key_generation_event;
use prost::Message;
use tea_actor_utility::{actor_env::get_my_tea_id, actor_pinner::is_node_ready};
use wascc_actor::prelude::codec::messaging::BrokerMessage;
use wascc_actor::HandlerResult;

pub fn key_generation_request_handler(msg: &BrokerMessage) -> HandlerResult<()> {
    let base64_decoded_msg_body = base64::decode(String::from_utf8(msg.body.clone())?)?;
    Ok(is_node_ready(crate::MY_ACTOR_NAME, move |ready| {
        if !ready {
            info!("i'm not ready, just ignore the key generation request message");
            return Ok(());
        }

        let key_generation_response = crate::actor_delegate_proto::KeyGenerationResponse::decode(
            base64_decoded_msg_body.as_slice(),
        )?;
        trace!(
            "KeyGeneratioResponse protobuf decoded {:?}",
            &key_generation_response
        );
        process_key_generation_event(key_generation_response)?;
        Ok(())
    })?)
}

pub fn sign_with_key_slices_handler(msg: &BrokerMessage) -> HandlerResult<()> {
    let base64_decoded_msg_body = base64::decode(String::from_utf8(msg.body.clone())?)?;
    Ok(is_node_ready(crate::MY_ACTOR_NAME, move |ready| {
        if !ready {
            debug!("node is not ready, just ignore layer1 SignWithKeySlicesRequested request");
            return Ok(());
        }

        let sign_with_key_slices_request =
            crate::actor_delegate_proto::SignWithKeySlicesRequest::decode(
                base64_decoded_msg_body.as_slice(),
            )?;
        trace!(
            "SignWithKeySlicesRequest protobuf decoded {:?}",
            &sign_with_key_slices_request,
        );

        let tea_id = get_my_tea_id()?;
        if tea_id.eq(&sign_with_key_slices_request.delegator_tea_id) {
            crate::delegator::process_sign_with_key_slices_event(
                sign_with_key_slices_request.clone(),
            )?;
        }

        crate::executor::process_sign_with_key_slices_event(sign_with_key_slices_request)?;
        Ok(())
    })?)
}
