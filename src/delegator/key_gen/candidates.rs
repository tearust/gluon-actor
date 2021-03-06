use crate::common::{send_key_candidate_request, TaskInfo};
use tea_actor_utility::actor_ipfs::ipfs_swarm_peers;

pub fn invite_candidate_initial_pinners(
    task_info: TaskInfo,
    filter_ids: Vec<String>,
) -> anyhow::Result<()> {
    let mut peers_ids: Vec<String> = ipfs_swarm_peers()?;
    peers_ids = peers_ids
        .into_iter()
        .filter(move |v| !filter_ids.contains(v))
        .collect();

    let candidates: Vec<String> =
        random_select_peers(peers_ids, task_info.exec_info.n, &task_info.task_id)
            .into_iter()
            .take(task_info.exec_info.n as usize * 2)
            .collect();
    for peer_id in candidates {
        send_key_candidate_request(&peer_id, task_info.clone(), false)?;
    }
    Ok(())
}

fn random_select_peers(ids: Vec<String>, n: u8, task_id: &str) -> Vec<String> {
    let candidates_count = n as u32 * 2;
    if ids.len() < candidates_count as usize {
        return ids;
    }
    let lucky_number = calculate_lucky_number(n, &task_id);

    let mut ids = ids;
    let mut distance = 0u8;
    let mut candidates_peers = Vec::<String>::new();
    while candidates_peers.len() < candidates_count as usize && !ids.is_empty() {
        ids = ids
            .into_iter()
            .filter(|item| {
                let lucky = calculate_lucky_number(n, item);
                if (lucky as i16 - lucky_number as i16).abs() as u8 <= distance {
                    candidates_peers.push(item.clone());
                    false
                } else {
                    true
                }
            })
            .collect();
        distance += 1;
    }
    candidates_peers
}

fn calculate_lucky_number(n: u8, id: &str) -> u8 {
    id.as_bytes()[id.as_bytes().len() - 1] % n
}

#[cfg(test)]
mod tests {
    use super::random_select_peers;

    #[test]
    fn random_select_peers_works() -> anyhow::Result<()> {
        let mut peers = Vec::<String>::new();
        for i in 0..=255u8 {
            peers.push(String::from(i as char));
        }
        let task_id = String::from(0u8 as char);

        // boundary test
        assert_eq!(256, random_select_peers(peers.clone(), 255, &task_id).len());
        assert_eq!(256, random_select_peers(peers.clone(), 1, &task_id).len());

        // normal test
        assert_eq!(128, random_select_peers(peers.clone(), 2, &task_id).len());
        assert_eq!(85, random_select_peers(peers.clone(), 3, &task_id).len());

        // double peers
        for i in 0..=255u8 {
            peers.push(String::from(i as char));
        }
        assert_eq!(512, random_select_peers(peers.clone(), 255, &task_id).len());
        assert_eq!(512, random_select_peers(peers.clone(), 1, &task_id).len());
        assert_eq!(256, random_select_peers(peers.clone(), 2, &task_id).len());
        assert_eq!(170, random_select_peers(peers.clone(), 3, &task_id).len());

        Ok(())
    }
}
