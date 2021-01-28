use super::task_info::TaskInfo;
use crate::BINDING_NAME;
use anyhow::anyhow;
use tea_actor_utility::{
    actor_env::get_my_ephemeral_id,
    actor_kvp,
    actor_util::{
        generate_rsa_keypair, rsa_decrypt, rsa_key_to_bytes, sign_ed25519_message,
        verify_ed25519_signature,
    },
    ipfs_p2p::send_message,
    layer1::lookup_node_profile,
};
use wascc_actor::HandlerResult;

pub const PREFIX_KEY_GEN_RSA_KEY: &'static str = "key_gen_rsa_key";

pub fn send_key_candidate_request(
    peer_id: &str,
    task_info: TaskInfo,
    executor: bool,
) -> anyhow::Result<()> {
    let task_id = task_info.task_id.clone();
    let n = task_info.exec_info.n as u32;
    let k = task_info.exec_info.k as u32;
    let key_type = task_info.exec_info.task_type.clone();
    let delegator_ephemeral_id = get_my_ephemeral_id().map_err(|e| anyhow::anyhow!("{}", e))?;
    let signature = sign_ed25519_message(
        &to_candidate_signature_bytes(&task_id, n, k, &key_type, &delegator_ephemeral_id, executor),
        None,
    )?;

    let req = crate::p2p_proto::KeyGenerationCandidateRequest {
        task_id,
        n,
        k,
        key_type,
        delegator_ephemeral_id,
        executor,
        signature,
    };
    debug!("begin send_key_candidate_request with params: {:?}", req);
    send_message(
        peer_id,
        &task_info.task_id,
        crate::p2p_proto::GeneralMsg {
            msg: Some(crate::p2p_proto::general_msg::Msg::KeyGenerationCandidateRequest(req)),
        },
    )
}

pub fn verify_to_candidate_signature<F>(
    peer_id: &str,
    req: &crate::p2p_proto::KeyGenerationCandidateRequest,
    mut callback: F,
) -> anyhow::Result<()>
where
    F: FnMut() -> HandlerResult<()> + Clone + Sync + Send + 'static,
{
    let raw = to_candidate_signature_bytes(
        &req.task_id,
        req.n,
        req.k,
        &req.key_type,
        &req.delegator_ephemeral_id,
        req.executor,
    );
    if !verify_ed25519_signature(
        req.delegator_ephemeral_id.clone(),
        raw,
        req.signature.clone(),
    )? {
        return Err(anyhow::anyhow!("invalid signature in candidate"));
    }

    let peer_id = peer_id.to_string();
    lookup_node_profile(
        &req.delegator_ephemeral_id,
        "actor.gluon.inbox",
        move |profile| {
            if !peer_id.eq(&profile.peer_id) {
                let msg = format!(
                    "invalid delegator: peer_id mismatch, expect is {}, actual is {}",
                    &profile.peer_id, &peer_id
                );
                return Err(msg.into());
            }
            callback()
        },
    )
    .map_err(|e| anyhow!("{}", e))?;
    Ok(())
}

fn to_candidate_signature_bytes(
    task_id: &str,
    n: u32,
    k: u32,
    key_type: &str,
    delegator_ephemeral_id: &[u8],
    executor: bool,
) -> Vec<u8> {
    let mut buf = task_id.as_bytes().to_vec();
    buf.extend(&n.to_le_bytes());
    buf.extend(&k.to_le_bytes());
    buf.extend(key_type.as_bytes());
    buf.extend(delegator_ephemeral_id);
    buf.push(match executor {
        true => 1u8,
        false => 0u8,
    });
    buf
}

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
