use portal::{key_generation_request_handler, sign_with_key_slices_handler};
use prost::Message;
use tea_actor_utility::actor_nats::response_reply_with_subject;
use tea_actor_utility::{action, encode_protobuf, ipfs_p2p, p2p_proto};
use wascc_actor::prelude::codec::messaging::BrokerMessage;
use wascc_actor::prelude::*;
use wascc_actor::HandlerResult;

mod common;
mod delegator;
mod executor;
mod initial_pinner;
mod pinner;
mod portal;
mod actor_delegate_proto {
    include!(concat!(env!("OUT_DIR"), "/actor_delegate.rs"));
}
mod actor_pinner_proto {
    include!(concat!(env!("OUT_DIR"), "/actor_pinner.rs"));
}

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

const BINDING_NAME: &'static str = "tea_gluon";
const MY_ACTOR_NAME: &'static str = "gluon";
const PINNER_ACTOR_NAME: &'static str = "pinner";

actor_handlers! {
    codec::messaging::OP_DELIVER_MESSAGE => handle_message,
    codec::core::OP_HEALTH_REQUEST => health
}

fn handle_message(msg: BrokerMessage) -> HandlerResult<()> {
    let channel_parts: Vec<&str> = msg.subject.split('.').collect();
    match &channel_parts[..] {
        ["ipfs", "p2p", "listen", from_peer_id] => listen_p2p_message(&from_peer_id, &msg),
        ["actor", "pinner", "event", "client_operation_after_verify"] => {
            pinner_client_operation_after_verify(&msg)
        }
        ["actor", "pinner", "event", "server_check_strategy"] => pinner_server_check_strategy(&msg),

        ["layer1", "event", "tea", "KeyGenerationRequested"] => {
            key_generation_request_handler(&msg)
        }
        ["layer1", "event", "tea", "SignWithKeySlicesRequested"] => {
            sign_with_key_slices_handler(&msg)
        }
        ["actor", "gluon", "inbox", uuid] => action::result_handler(&msg, uuid),
        ["reply", _actor, uuid] => action::result_handler(&msg, uuid),

        #[cfg(feature = "dev")]
        ["internal", "op", "debug", "key_gen_response_message"] => {
            delegator::dump_methods::generate_key_gen_response_message(&msg)
        }

        _ => Ok(()),
    }
}

fn health(_req: codec::core::HealthRequest) -> HandlerResult<()> {
    Ok(())
}

fn pinner_server_check_strategy(msg: &BrokerMessage) -> HandlerResult<()> {
    let res = crate::actor_pinner_proto::ServerCheckStrategy::decode(msg.body.as_slice())?;
    let item = res.item.ok_or(anyhow::anyhow!(
        "{}:{} expect a ChallangeStoreItem field",
        line!(),
        file!()
    ))?;
    if delegator::is_key_gen_tag(&item) || delegator::is_sign_tag(&item) {
        return Ok(response_reply_with_subject(
            "",
            &msg.reply_to,
            encode_protobuf(crate::actor_pinner_proto::ServerCheckStrategyResult {
                verify: true,
                message: "passed".to_string(),
            })?,
        )?);
    }
    Ok(())
}

fn pinner_client_operation_after_verify(msg: &BrokerMessage) -> HandlerResult<()> {
    let res = crate::actor_pinner_proto::ClientOperationAfterVerify::decode(msg.body.as_slice())?;
    let item = res.item.ok_or(anyhow::anyhow!(
        "{}:{} expect a ChallangeStoreItem field",
        line!(),
        file!()
    ))?;
    if delegator::is_key_gen_tag(&item) {
        delegator::key_gen_operation_after_verify_handler(
            res.peer_id,
            res.pinner_ephemeral_id,
            &item,
        )?;
    } else if delegator::is_sign_tag(&item) {
        delegator::sign_operation_after_verify_handler(
            res.peer_id,
            res.pinner_ephemeral_id,
            &item,
        )?;
    }

    Ok(())
}

fn listen_p2p_message(from_peer_id: &str, msg: &BrokerMessage) -> HandlerResult<()> {
    trace!("gluon actor got p2p message from {}", from_peer_id);
    Ok(ipfs_p2p::listen_message(
        &from_peer_id.clone(),
        &msg,
        move |g_msg, from_peer_id, reply_to| match g_msg.msg.clone() {
            Some(crate::p2p_proto::general_msg::Msg::TaskKeyGenerationApplyRequst(req)) => Ok(
                delegator::task_key_generation_apply_request_handler(req, from_peer_id, reply_to)?,
            ),
            Some(crate::p2p_proto::general_msg::Msg::TaskExecutionRequest(req)) => Ok(
                executor::task_execution_request_handler(req, from_peer_id, reply_to)?,
            ),
            Some(crate::p2p_proto::general_msg::Msg::TaskExecutionResponse(res)) => Ok(
                delegator::task_execution_response_handler(res, from_peer_id, reply_to)?,
            ),
            Some(crate::p2p_proto::general_msg::Msg::TaskPinnerKeySliceRequest(req)) => {
                Ok(initial_pinner::task_pinner_key_slice_request_handler(
                    req,
                    from_peer_id.into(),
                    reply_to.into(),
                )?)
            }
            Some(crate::p2p_proto::general_msg::Msg::TaskPinnerKeySliceResponse(res)) => Ok(
                delegator::task_pinner_key_slice_response_handler(res, from_peer_id, reply_to)?,
            ),
            Some(crate::p2p_proto::general_msg::Msg::TaskSignWithKeySlicesRequst(req)) => Ok(
                delegator::task_sign_with_key_slices_request_handler(req, from_peer_id, reply_to)?,
            ),
            Some(crate::p2p_proto::general_msg::Msg::TaskSignWithKeySlicesResponse(res)) => Ok(
                executor::task_sign_with_key_slices_response_handler(res, from_peer_id, reply_to)?,
            ),
            Some(crate::p2p_proto::general_msg::Msg::TaskSignGetPinnerKeySliceRequest(req)) => {
                Ok(pinner::task_sign_with_key_slices_request_handler(
                    req,
                    from_peer_id.to_string(),
                    reply_to.to_string(),
                )?)
            }
            Some(crate::p2p_proto::general_msg::Msg::TaskSignGetPinnerKeySliceResponse(res)) => {
                Ok(delegator::task_sign_get_pinner_key_slice_response_handler(
                    res,
                    from_peer_id,
                    reply_to,
                )?)
            }
            Some(crate::p2p_proto::general_msg::Msg::TaskCommitSignResultRequest(req)) => Ok(
                delegator::task_commit_sign_result_request_handler(req, from_peer_id, reply_to)?,
            ),
            Some(crate::p2p_proto::general_msg::Msg::KeyGenerationCandidateRequest(req)) => {
                Ok(match req.executor {
                    true => executor::task_key_generation_candidate_request_handler(
                        from_peer_id.to_string(),
                        req,
                    ),
                    false => initial_pinner::task_key_generation_candidate_request_handler(
                        from_peer_id.to_string(),
                        req,
                    ),
                }?)
            }
            _ => {
                debug!("Task actor unhandled p2p message type");
                Ok(response_reply_with_subject(
                    "",
                    &reply_to,
                    "Task actor unknown message".as_bytes().to_vec(),
                )?)
            }
        },
    )?)
}
