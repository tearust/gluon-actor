use crate::initial_pinner::store_item::StoreItemState;
use crate::{
    common::{decrypt_key_slice, send_key_generation_request, verify_to_candidate_signature},
    executor::ExecutorStoreItem,
};
use serde::export::TryFrom;
use tea_actor_utility::actor_crypto::{aes_encrypt, generate_aes_key};
use tea_actor_utility::actor_ipfs::ipfs_block_put;
use tea_actor_utility::actor_util::{rsa_encrypt, rsa_key_to_bytes};
use tea_actor_utility::{
    action,
    actor_nats::response_reply_with_subject,
    encode_protobuf,
    ipfs_p2p::{log_and_response_with_error, send_message},
};
use wascc_actor::prelude::codec::messaging::BrokerMessage;
use wascc_actor::HandlerResult;

pub use super::store_item::InitialPinnerStoreItem;

pub fn task_pinner_key_slice_request_handler(
    req: crate::p2p_proto::TaskPinnerKeySliceRequest,
    peer_id: String,
    reply_to: String,
) -> anyhow::Result<()> {
    match trying_get_initial_pinner_store_item(&req.task_id) {
        Ok(mut item) => {
            item.state = StoreItemState::Responded;
            InitialPinnerStoreItem::save(&item)?;

            deploy_key_slice(req.clone(), move |deployment_id| {
                item.state = StoreItemState::Deployed;
                InitialPinnerStoreItem::save(&item)?;

                send_message(
                    &peer_id,
                    &req.task_id,
                    crate::p2p_proto::GeneralMsg {
                        msg: Some(
                            crate::p2p_proto::general_msg::Msg::TaskPinnerKeySliceResponse(
                                crate::p2p_proto::TaskPinnerKeySliceResponse {
                                    task_id: req.task_id.clone(),
                                    deployment_id,
                                },
                            ),
                        ),
                    },
                )?;
                Ok(response_reply_with_subject(
                    "",
                    &reply_to,
                    "pinned key slice successfully".as_bytes().to_vec(),
                )?)
            })
        }
        Err(e) => log_and_response_with_error(
            &reply_to,
            &peer_id,
            &req.task_id,
            &format!(
                "failed to get InitialPinnerStoreItem with task_id {}, details: {}",
                &req.task_id, e
            ),
        ),
    }
}

fn deploy_key_slice<F>(
    req: crate::p2p_proto::TaskPinnerKeySliceRequest,
    callback: F,
) -> anyhow::Result<()>
where
    F: FnMut(String) -> HandlerResult<()> + Clone + Sync + Send + 'static,
{
    let session_id = wascc_actor::extras::default()
        .get_guid()
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let subject = format!(
        "actor.pinner.intercom.register_upload_rsa_key.{}",
        &session_id
    );
    Ok(action::call_async_intercom(
        crate::PINNER_ACTOR_NAME,
        crate::MY_ACTOR_NAME,
        BrokerMessage {
            subject,
            reply_to: "".into(),
            body: Vec::new(),
        },
        move |msg| {
            let key_slice = decrypt_key_slice(&req.task_id, req.encrypted_key_slice.clone())?;
            let key1 = generate_aes_key()?;
            let encrypted_data = aes_encrypt(key1.clone(), key_slice)?;
            let (data_cid, _) = ipfs_block_put(&encrypted_data, true)?;

            let pk_str = String::from_utf8(msg.body.clone())?;
            let rsa_pub_key = rsa_key_to_bytes(pk_str)?;
            let subject = format!(
                "actor.pinner.intercom.data_upload_completed_process.{}",
                &session_id
            );
            let to_value = |value: String| Some(crate::actor_pinner_proto::StringValue { value });
            let mut callback = callback.clone();
            action::call_async_intercom(
                crate::PINNER_ACTOR_NAME,
                crate::MY_ACTOR_NAME,
                BrokerMessage {
                    subject,
                    reply_to: "".into(),
                    body: encode_protobuf(
                        crate::actor_pinner_proto::DataUploadCompletedProcessRequest {
                            cid_code: to_value(data_cid),
                            cid_description: to_value("".into()), // todo add description about key slice
                            cid_capchecker: to_value("".into()),
                            key_url_encoded: to_value(base64::encode(rsa_encrypt(
                                rsa_pub_key,
                                key1,
                            )?)),
                        },
                    )?,
                },
                move |msg| {
                    debug!("data_upload_completed_process got response: {:?}", msg);
                    callback(String::from_utf8(msg.body.clone())?)
                },
            )?;
            Ok(())
        },
    )
    .map_err(|e| anyhow::anyhow!("{}", e))?)
}

pub fn task_key_generation_candidate_request_handler(
    peer_id: String,
    req: crate::p2p_proto::KeyGenerationCandidateRequest,
) -> anyhow::Result<()> {
    trace!(
        "initial pinner received KeyGenerationCandidateRequest: {:?}",
        req
    );
    verify_to_candidate_signature(&peer_id.clone(), &req.clone(), move || {
        let mut store_item = InitialPinnerStoreItem::try_from(req.clone())?;
        if !willing_to_run(&store_item) {
            info!(
                "I'm not willing to run {}, just ignore",
                &store_item.task_info.task_id
            );
            return Ok(());
        }

        if let Err(e) = check_capabilities(&store_item) {
            info!(
                "do not have capabilities to be executor of {}, details: {}",
                &store_item.task_info.task_id, e
            );
            return Ok(());
        }
        InitialPinnerStoreItem::save(&store_item)?;

        send_key_generation_request(&peer_id, &store_item.task_info, false)?;
        store_item.state = StoreItemState::Requested;
        InitialPinnerStoreItem::save(&store_item)?;
        Ok(())
    })
}

fn willing_to_run(_item: &InitialPinnerStoreItem) -> bool {
    // TODO check errand payment plan and i'm willing to run it
    true
}

fn check_capabilities(_item: &InitialPinnerStoreItem) -> anyhow::Result<()> {
    // todo check if capabilities of my tea-box meets the request of task_info
    Ok(())
}

fn trying_get_initial_pinner_store_item(task_id: &str) -> anyhow::Result<InitialPinnerStoreItem> {
    match InitialPinnerStoreItem::get(task_id) {
        Ok(item) => Ok(item),
        Err(_) => match ExecutorStoreItem::get(task_id) {
            Ok(item) => Ok(item.into()),
            Err(e) => Err(e),
        },
    }
}
