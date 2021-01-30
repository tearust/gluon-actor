mod handler;
mod store_item;

pub use handler::{task_pinner_key_slice_request_handler, InitialPinnerStoreItem};

pub fn task_key_generation_candidate_request_handler(
    peer_id: String,
    req: crate::p2p_proto::KeyGenerationCandidateRequest,
) -> anyhow::Result<()> {
    if super::executor::ExecutorStoreItem::contains(&req.task_id)? {
        info!("i have processed executor candidate request already, just ignore this");
        return Ok(());
    }
    handler::task_key_generation_candidate_request_handler(peer_id, req)
}
