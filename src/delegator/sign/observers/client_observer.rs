use crate::delegator::{
    executor_info::ExecutorInfo,
    sign::{
        ra::{
            is_executor_ra_response, is_pinner_ra_response, PROPERTY_RSA_PUB_KEY, PROPERTY_TASK_ID,
        },
        store_item::DelegatorSignStoreItem,
        try_send_to_executor,
    },
};
use tea_actor_utility::ipfs_p2p::send_message;

// this property value set in pinner actor in response_peer_approve_pinner_handler method
const PROPERTY_KEY_DEPLOYMENT_ID: &'static str = "deployment_id";

pub fn operation_after_verify_handler(
    peer_id: String,
    _ephemeral_id: Vec<u8>,
    item: &crate::actor_pinner_proto::ChallangeStoreItem,
) -> anyhow::Result<()> {
    let task_id = &item
        .properties
        .iter()
        .find(|v| PROPERTY_TASK_ID.eq(&v.key))
        .ok_or(anyhow::anyhow!(
            "{}:{} failed to get task id from ChallengeStoreItem {}",
            line!(),
            file!(),
            &item.uuid
        ))?
        .value;

    if is_executor_ra_response(item) {
        let rsa_pub_key = base64::decode(
            item.properties
                .iter()
                .find(|v| PROPERTY_RSA_PUB_KEY.eq(&v.key))
                .ok_or(anyhow::anyhow!(
                    "{}:{} failed to get ras public key from ChallengeStoreItem {}",
                    line!(),
                    file!(),
                    &item.uuid
                ))?
                .value
                .clone(),
        )?;

        on_executor_ra_success(task_id, &peer_id, rsa_pub_key)?;
    } else if is_pinner_ra_response(item) {
        let deployment_id = &item
            .properties
            .iter()
            .find(|v| PROPERTY_KEY_DEPLOYMENT_ID.eq(&v.key))
            .ok_or(anyhow::anyhow!(
                "{}:{} failed to get deployment_id id from ChallengeStoreItem {}",
                line!(),
                file!(),
                &item.uuid
            ))?
            .value;
        on_pinner_ra_success(task_id, &peer_id, deployment_id)?;
    }
    Ok(())
}

fn on_executor_ra_success(
    task_id: &str,
    peer_id: &str,
    rsa_pub_key: Vec<u8>,
) -> anyhow::Result<()> {
    debug!("got sign executor ra success response, task id: {}, peer id: {}", task_id, peer_id);

    let mut store_item = DelegatorSignStoreItem::get(task_id)?;
    if store_item.executor.is_some() {
        info!("executor already exists, just ignore");
        return Ok(());
    }

    store_item.executor = Some(ExecutorInfo {
        peer_id: peer_id.to_string(),
        rsa_pub_key,
    });
    let candidates = store_item.pop_all_candidates();
    DelegatorSignStoreItem::save(&store_item)?;

    if store_item.ready_send_to_executor() {
        return try_send_to_executor(&mut store_item);
    }

    for (deployment_id, peers) in candidates {
        for peer_id in peers {
            send_get_pinner_key_slice_request(task_id, &peer_id, &deployment_id, &store_item)?;
        }
    }
    Ok(())
}

fn on_pinner_ra_success(task_id: &str, peer_id: &str, deployment_id: &str) -> anyhow::Result<()> {
    debug!("go sign pinner ra success response, task id: {}, peer id: {}", task_id, peer_id);

    let mut store_item = DelegatorSignStoreItem::get(task_id)?;
    if store_item.has_found_key_slice(deployment_id) {
        info!(
            "key_slice with deployment_id {} already exists, just ignore",
            deployment_id
        );
        return Ok(());
    }

    if store_item.executor.is_none() {
        info!("executor not ready, deal later");

        store_item.insert_deployment(deployment_id, peer_id)?;
        DelegatorSignStoreItem::save(&store_item)?;
        return Ok(());
    }

    send_get_pinner_key_slice_request(task_id, peer_id, deployment_id, &store_item)
}

fn send_get_pinner_key_slice_request(
    task_id: &str,
    peer_id: &str,
    deployment_id: &str,
    item: &DelegatorSignStoreItem,
) -> anyhow::Result<()> {
    debug!("begin to send get pinner key slice request to {}, task id is: {}", peer_id, task_id);
    let req = crate::p2p_proto::GeneralMsg {
        msg: Some(
            crate::p2p_proto::general_msg::Msg::TaskSignGetPinnerKeySliceRequest(
                crate::p2p_proto::TaskSignGetPinnerKeySliceRequest {
                    task_id: task_id.to_string(),
                    rsa_pub_key: item.executor.as_ref().unwrap().rsa_pub_key.clone(),
                    deployment_id: deployment_id.to_string(),
                },
            ),
        ),
    };
    send_message(peer_id, task_id, req)
}
