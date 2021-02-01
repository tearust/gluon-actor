use crate::delegator::{
    executor_info::ExecutorInfo,
    key_gen::{
        initial_pinner_info::InitialPinnerInfo,
        ra::{
            is_executor_ra_response, is_initial_pinner_ra_response, PROPERTY_RSA_PUB_KEY,
            PROPERTY_TASK_ID,
        },
        store_item::{DelegatorKeyGenStoreItem, StoreItemState},
        try_send_to_executor, TaskCandidates,
    },
};

pub fn operation_after_verify_handler(
    peer_id: String,
    _ephemeral_id: Vec<u8>,
    item: &crate::actor_pinner_proto::ChallangeStoreItem,
) -> anyhow::Result<()> {
    debug!("operation_after_verify_handler item: {:?}", item);
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

    if is_executor_ra_response(item) {
        on_executor_ra_success(&task_id, &peer_id, rsa_pub_key)?;
    } else if is_initial_pinner_ra_response(item) {
        on_initial_pinner_ra_success(&task_id, &peer_id, rsa_pub_key)?;
    }
    Ok(())
}

pub fn on_executor_ra_success(
    task_id: &str,
    peer_id: &str,
    rsa_pub_key: Vec<u8>,
) -> anyhow::Result<()> {
    let mut store_item = DelegatorKeyGenStoreItem::get(task_id)?;
    if store_item.state != StoreItemState::InvitedCandidates {
        debug!("executor RA response ignored because state is: {:?}", &store_item.state);
        return Ok(());
    }

    debug!("validate executor {} successfully", peer_id);
    store_item.insert_executor(ExecutorInfo {
        peer_id: peer_id.to_string(),
        rsa_pub_key,
    });
    DelegatorKeyGenStoreItem::save(&store_item)?;

    try_send_to_executor(&mut store_item)
}

pub fn on_initial_pinner_ra_success(
    task_id: &str,
    peer_id: &str,
    rsa_pub_key: Vec<u8>,
) -> anyhow::Result<()> {
    let mut store_item = DelegatorKeyGenStoreItem::get(task_id)?;
    if store_item.state != StoreItemState::InvitedCandidates {
        debug!("initial pinner RA response ignored because state is: {:?}", &store_item.state);
        return Ok(());
    }

    debug!("validate initial pinner {} successfully", peer_id);
    store_item.insert_initial_pinner(InitialPinnerInfo {
        peer_id: peer_id.to_string(),
        rsa_pub_key,
    });
    DelegatorKeyGenStoreItem::save(&store_item)?;

    try_send_to_executor(&mut store_item)
}
