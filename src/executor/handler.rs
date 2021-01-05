pub fn process_key_generation_event(
    res: crate::actor_delegate_proto::KeyGenerationResponse,
) -> anyhow::Result<bool> {
    super::key_gen::process_key_generation_event(res)
}

pub fn task_execution_request_handler(
    request: crate::p2p_proto::TaskExecutionRequest,
    peer_id: &str,
    reply_to: &str,
) -> anyhow::Result<()> {
    super::key_gen::task_execution_request_handler(request, peer_id, reply_to)
}

pub fn task_sign_with_key_slices_response_handler(
    request: crate::p2p_proto::TaskSignWithKeySlicesResponse,
    peer_id: &str,
    reply_to: &str,
) -> anyhow::Result<()> {
    super::sign::task_sign_with_key_slices_response_handler(request, peer_id, reply_to)
}

pub fn process_sign_with_key_slices_event(
    req: crate::actor_delegate_proto::SignWithKeySlicesRequest,
) -> anyhow::Result<()> {
    super::sign::process_sign_with_key_slices_event(req)
}
