use crate::common::{send_key_generation_request, verify_to_candidate_signature};
use crate::executor::store_item::{ExecutorStoreItem, StoreItemState};
use serde::export::TryFrom;
use tea_actor_utility::{
    actor_crypto,
    actor_crypto::generate_multi_sig_asset,
    actor_nats::response_reply_with_subject,
    actor_util::rsa_encrypt,
    ipfs_p2p::{log_and_response_with_error, send_message},
};

pub fn task_key_generation_candidate_request_handler(
    peer_id: String,
    req: crate::p2p_proto::KeyGenerationCandidateRequest,
) -> anyhow::Result<()> {
    verify_to_candidate_signature(&peer_id.clone(), &req.clone(), move || {
        let mut store_item = ExecutorStoreItem::try_from(req.clone())?;

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

        send_key_generation_request(&peer_id, &store_item.task_info, true)?;
        store_item.state = StoreItemState::Requested;
        ExecutorStoreItem::save(&store_item)?;
        Ok(())
    })
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

pub fn task_execution_request_handler(
    request: crate::p2p_proto::TaskExecutionRequest,
    peer_id: &str,
    reply_to: &str,
) -> anyhow::Result<()> {
    match ExecutorStoreItem::get(&request.task_id) {
        Ok(mut item) => {
            item.state = StoreItemState::Responded;
            ExecutorStoreItem::save(&item)?;

            let res = generate_task_execution_response(&item, &request)?;
            send_message(
                peer_id,
                &item.task_info.task_id,
                crate::p2p_proto::GeneralMsg {
                    msg: Some(crate::p2p_proto::general_msg::Msg::TaskExecutionResponse(
                        res,
                    )),
                },
            )?;
            item.state = StoreItemState::Executed;
            ExecutorStoreItem::save(&item)?;

            response_reply_with_subject(
                "",
                reply_to,
                "response task execution request successfully"
                    .as_bytes()
                    .to_vec(),
            )
        }
        Err(e) => log_and_response_with_error(
            reply_to,
            peer_id,
            &request.task_id,
            &format!(
                "failed to get ExecutorStoreItem with task_id {}, details: {}",
                &request.task_id, e
            ),
        ),
    }
}

pub fn generate_task_execution_response(
    item: &ExecutorStoreItem,
    request: &crate::p2p_proto::TaskExecutionRequest,
) -> anyhow::Result<crate::p2p_proto::TaskExecutionResponse> {
    let (pk, sk) = generate_key_by_type(&request.key_type)?;
    let key_slices = actor_crypto::shamir_share(
        request.initial_pinners.len() as u8,
        request.minimum_recovery_number as u8,
        sk,
    )?;

    let mut index = 0;
    let mut initial_pinners: Vec<crate::p2p_proto::TaskResultInitialPinnerData> = Vec::new();
    for pinner_data in request.initial_pinners.iter() {
        initial_pinners.push(crate::p2p_proto::TaskResultInitialPinnerData {
            peer_id: pinner_data.peer_id.clone(),
            encrypted_key_slice: rsa_encrypt(
                pinner_data.rsa_pub_key.clone(),
                key_slices[index].clone(),
            )?,
        });
        index += 1;
    }

    let p1 = request.p1_public_key.clone();
    let multi_sig_account = generate_multi_sig_account(
        &p1,
        &pk,
        None,
        item.task_info.exec_info.k,
        &item.task_info.exec_info.task_type,
    )?;

    Ok(crate::p2p_proto::TaskExecutionResponse {
        task_id: request.task_id.clone(),
        initial_pinners,
        p2_public_key: pk,
        multi_sig_account,
    })
}

fn generate_multi_sig_account(
    p1: &[u8],
    p2: &[u8],
    p3: Option<Vec<u8>>,
    k: u8,
    task_type: &str,
) -> anyhow::Result<Vec<u8>> {
    let mut public_keys = vec![p1.to_vec(), p2.to_vec()];
    if let Some(p3) = p3 {
        public_keys.push(p3);
    }
    Ok(generate_multi_sig_asset(k, public_keys, task_type.to_string())?.into_bytes())
}

fn generate_key_by_type(key_type: &str) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    actor_crypto::generate(key_type.to_string())
}
