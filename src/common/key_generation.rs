use super::task_info::TaskInfo;
use crate::BINDING_NAME;
use anyhow::anyhow;
use tea_actor_utility::{
    actor_kvp,
    actor_util::{generate_rsa_keypair, rsa_decrypt, rsa_key_to_bytes},
    ipfs_p2p::send_message,
};

pub const PREFIX_KEY_GEN_RSA_KEY: &'static str = "key_gen_rsa_key";

pub fn send_key_generation_request(
    peer_id: &str,
    task_info: &TaskInfo,
    apply_executor: bool,
) -> anyhow::Result<()> {
    let rsa_key_pkcs1 = generate_rsa_keypair()?;
    actor_kvp::set(
        BINDING_NAME,
        &get_rsa_encrypt_key(&task_info.task_id),
        &rsa_key_pkcs1.private_key,
        6000,
    )?;

    let req = crate::p2p_proto::TaskKeyGenerationApplyRequst {
        task_id: task_info.task_id.clone(),
        rsa_pub_key: rsa_key_to_bytes(rsa_key_pkcs1.public_key)?,
        cap_desc: None,
        apply_executor,
    };

    send_message(
        peer_id,
        &task_info.task_id,
        crate::p2p_proto::GeneralMsg {
            msg: Some(crate::p2p_proto::general_msg::Msg::TaskKeyGenerationApplyRequst(req)),
        },
    )
}

pub fn decrypt_key_slice(task_id: &str, key_slice_encrypted: Vec<u8>) -> anyhow::Result<Vec<u8>> {
    let rsa_priv_key: String =
        actor_kvp::get(BINDING_NAME, &get_rsa_encrypt_key(task_id))?.ok_or(anyhow!(
            "{}:{} could not find rsa pub key {} corresponding rsa private",
            line!(),
            file!(),
            task_id
        ))?;

    let key_slice = rsa_decrypt(rsa_key_to_bytes(rsa_priv_key)?, key_slice_encrypted)?;
    Ok(key_slice)
}

fn get_rsa_encrypt_key(task_id: &str) -> String {
    format!("{}_{}", PREFIX_KEY_GEN_RSA_KEY, task_id)
}
