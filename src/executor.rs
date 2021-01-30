mod handler;
mod key_gen;
mod sign;
mod store_item;

pub use handler::{
    process_sign_with_key_slices_event, task_execution_request_handler,
    task_sign_with_key_slices_response_handler, ExecutorStoreItem,
};

pub fn task_key_generation_candidate_request_handler(
    peer_id: String,
    req: crate::p2p_proto::KeyGenerationCandidateRequest,
) -> anyhow::Result<()> {
    if super::initial_pinner::InitialPinnerStoreItem::contains(&req.task_id)? {
        info!("i have processed initial pinner candidate request already, just ignore this");
        return Ok(());
    }
    handler::task_key_generation_candidate_request_handler(peer_id, req)
}
