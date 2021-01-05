pub fn task_pinner_key_slice_response_handler(
    res: crate::p2p_proto::TaskPinnerKeySliceResponse,
    peer_id: &str,
    reply_to: &str,
) -> anyhow::Result<()> {
    super::key_gen::process_task_pinner_key_slice_response(res, peer_id, reply_to)
}

pub fn task_sign_with_key_slices_request_handler(
    res: crate::p2p_proto::TaskSignWithKeySlicesRequst,
    peer_id: &str,
    reply_to: &str,
) -> anyhow::Result<()> {
    super::sign::process_executor_sign_with_key_slices_request(res, peer_id, reply_to)
}

pub fn task_sign_get_pinner_key_slice_response_handler(
    res: crate::p2p_proto::TaskSignGetPinnerKeySliceResponse,
    peer_id: &str,
    reply_to: &str,
) -> anyhow::Result<()> {
    super::sign::process_pinner_key_slice_response(res, peer_id, reply_to)
}

pub fn task_commit_sign_result_request_handler(
    req: crate::p2p_proto::TaskCommitSignResultRequest,
    peer_id: &str,
    reply_to: &str,
) -> anyhow::Result<()> {
    super::sign::process_commit_sign_result_request(req, peer_id, reply_to)
}

pub fn task_execution_response_handler(
    res: crate::p2p_proto::TaskExecutionResponse,
    peer_id: &str,
    reply_to: &str,
) -> anyhow::Result<()> {
    super::key_gen::process_task_execution_response(res, peer_id, reply_to)
}

pub fn task_key_generation_apply_request_handler(
    request: crate::p2p_proto::TaskKeyGenerationApplyRequst,
    peer_id: &str,
    reply_to: &str,
) -> anyhow::Result<()> {
    if request.apply_executor {
        super::key_gen::remote_attestation_executor(
            request,
            peer_id.to_string(),
            reply_to.to_string(),
        )?;
    } else {
        super::key_gen::remote_attestation_initial_pinner(
            request,
            peer_id.to_string(),
            reply_to.to_string(),
        )?;
    }
    Ok(())
}

pub fn process_key_generation_event(
    res: crate::actor_delegate_proto::KeyGenerationResponse,
) -> anyhow::Result<()> {
    super::key_gen::process_key_generation_event(res)
}

pub fn process_sign_with_key_slices_event(
    req: crate::actor_delegate_proto::SignWithKeySlicesRequest,
) -> anyhow::Result<()> {
    super::sign::process_sign_with_key_slices_event(req)
}
