use crate::executor::ExecutorStoreItem;
use tea_actor_utility::{
    actor_crypto::aes_decrypt, actor_ipfs::ipfs_block_get, actor_nats::response_reply_with_subject,
    actor_pinner::get_deployment_info, actor_util::rsa_encrypt, ipfs_p2p::send_message,
};

pub fn task_sign_with_key_slices_request_handler(
    req: crate::p2p_proto::TaskSignGetPinnerKeySliceRequest,
    peer_id: String,
    reply_to: String,
) -> anyhow::Result<()> {
    let deployment_id = req.deployment_id.clone();
    get_deployment_info(
        crate::MY_ACTOR_NAME,
        &deployment_id.clone(),
        move |data_cid, _, key1| {
            let key1 = key1.ok_or(anyhow::anyhow!("failed to get key1 of {}", &deployment_id))?;
            let data_cid =
                data_cid.ok_or(anyhow::anyhow!("failed to get key1 of {}", &deployment_id))?;

            let encrypted_key_slice = ipfs_block_get(&data_cid)?;
            let key_slice = aes_decrypt(key1, encrypted_key_slice)?;

            let encrypted_key_slice: Vec<u8> = rsa_encrypt(req.rsa_pub_key.clone(), key_slice)?;
            send_message(
                &peer_id,
                &req.task_id,
                crate::p2p_proto::GeneralMsg {
                    msg: Some(
                        crate::p2p_proto::general_msg::Msg::TaskSignGetPinnerKeySliceResponse(
                            crate::p2p_proto::TaskSignGetPinnerKeySliceResponse {
                                task_id: req.task_id.clone(),
                                encrypted_key_slice,
                                deployment_id: deployment_id.clone(),
                            },
                        ),
                    ),
                },
            )?;
            response_reply_with_subject("", &reply_to, "key slice returned".as_bytes().to_vec())?;
            Ok(())
        },
    )
}
