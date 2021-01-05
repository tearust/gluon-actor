use crate::executor::store_item::{ExecutorStoreItem, StoreItemState};
use crate::BINDING_NAME;
use std::convert::TryFrom;
use tea_actor_utility::{
    actor_crypto,
    actor_crypto::combine_to_witness,
    actor_kvp,
    actor_nats::response_reply_with_subject,
    actor_util::{generate_rsa_keypair, ras_keys_to_bytes, rsa_decrypt},
    ipfs_p2p::send_message,
    layer1::lookup_node_profile_by_tea_id,
};

pub const PREFIX_SIGN_RSA_KEY: &'static str = "sign_rsa_key";

pub fn task_sign_with_key_slices_response_handler(
    request: crate::p2p_proto::TaskSignWithKeySlicesResponse,
    peer_id: &str,
    reply_to: &str,
) -> anyhow::Result<()> {
    let task_id = request.task_id.clone();
    let mut item = ExecutorStoreItem::get(&task_id)?;
    item.state = StoreItemState::Responded;
    ExecutorStoreItem::save(&item)?;

    let mut key_slices: Vec<Vec<u8>> = Vec::new();
    for encrypted_key_slice in request.encrypted_key_slices {
        key_slices.push(decrypt_key_slice(&task_id, encrypted_key_slice)?);
    }

    let p2_private_key: Vec<u8> =
        actor_crypto::shamir_recovery(item.task_info.exec_info.k, key_slices)?;

    let p2_signature: Vec<u8> =
        actor_crypto::sign(request.key_type, p2_private_key, request.adhoc_data)?;

    // todo query p1, p2, p3 from layer1
    let public_keys: Vec<Vec<u8>> = vec![];
    let signatures = vec![p2_signature];
    let witness = combine_to_witness(
        item.task_info.exec_info.k,
        public_keys,
        signatures,
        item.task_info.exec_info.task_type.clone(),
    )?;

    let req = crate::p2p_proto::GeneralMsg {
        msg: Some(
            crate::p2p_proto::general_msg::Msg::TaskCommitSignResultRequest(
                crate::p2p_proto::TaskCommitSignResultRequest {
                    task_id: task_id.clone(),
                    witness,
                },
            ),
        ),
    };
    send_message(peer_id, &task_id, req)?;

    item.state = StoreItemState::Executed;
    ExecutorStoreItem::save(&item)?;
    response_reply_with_subject("", reply_to, "signed successfully".as_bytes().to_vec())
}

pub fn process_sign_with_key_slices_event(
    req: crate::actor_delegate_proto::SignWithKeySlicesRequest,
) -> anyhow::Result<()> {
    let delegator_tea_id = req.delegator_tea_id.clone();
    let mut store_item = ExecutorStoreItem::try_from(req)?;

    // todo query ExecutionInfo from layer1 and update store item

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
    ExecutorStoreItem::save(&store_item)?;

    let task_info = store_item.task_info.clone();
    lookup_node_profile_by_tea_id(&delegator_tea_id, "actor.delegate.inbox", move |profile| {
        send_sign_request(&profile.peer_id, &task_info.task_id)?;
        Ok(())
    })
    .map_err(|e| anyhow::anyhow!("{}", e))?;

    store_item.state = StoreItemState::Requested;
    ExecutorStoreItem::save(&store_item)?;
    Ok(())
}

fn willing_to_run(_item: &ExecutorStoreItem) -> bool {
    // TODO check errand payment plan and i'm willing to run it
    true
}

fn check_capabilities(_item: &ExecutorStoreItem) -> anyhow::Result<()> {
    // todo check if item.task_info.code_cid has deployed

    // todo check if capabilities of my tea-box meets the request of task_info
    Ok(())
}

fn send_sign_request(peer_id: &str, task_id: &str) -> anyhow::Result<()> {
    let rsa_key_pkcs1 = generate_rsa_keypair()?;
    actor_kvp::set(
        BINDING_NAME,
        &get_rsa_encrypt_key(task_id),
        &rsa_key_pkcs1.private_key,
        6000,
    )?;

    let req = crate::p2p_proto::TaskSignWithKeySlicesRequst {
        task_id: task_id.to_string(),
        rsa_pub_key: ras_keys_to_bytes(rsa_key_pkcs1.public_key)?,
        cap_desc: None,
    };

    send_message(
        peer_id,
        &task_id,
        crate::p2p_proto::GeneralMsg {
            msg: Some(crate::p2p_proto::general_msg::Msg::TaskSignWithKeySlicesRequst(req)),
        },
    )
}

fn get_rsa_encrypt_key(task_id: &str) -> String {
    format!("{}_{}", PREFIX_SIGN_RSA_KEY, task_id)
}

fn decrypt_key_slice(task_id: &str, key_slice_encrypted: Vec<u8>) -> anyhow::Result<Vec<u8>> {
    let rsa_priv_key: String =
        actor_kvp::get(BINDING_NAME, &get_rsa_encrypt_key(task_id))?.ok_or(anyhow::anyhow!(
            "{}:{} could not find rsa pub key {} corresponding rsa private",
            line!(),
            file!(),
            task_id
        ))?;

    let key_slice = rsa_decrypt(ras_keys_to_bytes(rsa_priv_key)?, key_slice_encrypted)?;
    Ok(key_slice)
}
