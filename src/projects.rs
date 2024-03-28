use bytemuck::Pod;
use bytemuck::Zeroable;
use clap::ArgMatches;
use risc0_zkvm::ExecutorEnv;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Copy, Clone, Pod, Serialize, Deserialize, Zeroable, Debug, PartialEq)]
#[repr(C)]
struct Felt([u64; 4]);

pub fn aggregate_project_votes(
    matches: &ArgMatches,
    votes_table: BTreeMap<String, Vec<i128>>,
    inputs: Vec<(Vec<u8>, Vec<u8>)>,
) -> ExecutorEnv<'_> {
    // filter by ballot count, with a "," as the delimiter
    let filter_by_ballot_count = matches
        .value_of("filter_by_amounts")
        .map(|s| {
            let mut iter = s.split(",");
            let min = iter.next().unwrap().parse::<usize>().unwrap();
            let max = iter.next().unwrap().parse::<usize>().unwrap();
            (min, max)
        })
        .unwrap_or((0, 0));

    // project ids to filter by in guest according to the filter_by_ballot_count votes range
    let mut project_ids: Vec<String> = vec![];

    // determine which projects ids have a vote count within the specified range
    if filter_by_ballot_count != (0, 0) {
        let vote_table_length = votes_table.len();
        let votes_table_filtered: BTreeMap<String, Vec<i128>> = votes_table
            .into_iter()
            .filter(|(_, amounts)| {
                let count = amounts.len();
                count >= filter_by_ballot_count.0 && count <= filter_by_ballot_count.1
            })
            .collect();
        if votes_table_filtered.len() == 0 {
            panic!("No projects have a vote count within the specified range")
        }
        // if the vote table length is equal to the post filter votes table length, then the filter is redundant and we should leave the
        // project_ids array empty when passed to guest
        if vote_table_length != votes_table_filtered.len() {
            project_ids.extend(votes_table_filtered.keys().cloned());
        }
        println!("project_ids filter: {:?}", project_ids);
    }

    let env: ExecutorEnv<'_> = ExecutorEnv::builder()
        .write(&inputs)
        .expect("Failed to deserialize inputs")
        .write(&project_ids)
        .expect("Failed to deserialize project_ids")
        .build()
        .unwrap();
    env
}
