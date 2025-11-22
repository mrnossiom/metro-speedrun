use std::{collections::HashMap, fs, path::PathBuf};

use clap::Parser;
use petgraph::{
	Graph,
	graph::{EdgeIndex, NodeIndex},
	visit::EdgeRef,
};

mod network;
mod queries;
mod traversals;

use crate::{
	network::{Network, traformed_to_dot},
	queries::types::{LineQid, StationQid},
};

#[derive(clap::Parser)]
struct Args {
	#[clap(default_value = "output")]
	outdir: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args = Args::parse();

	let mut network = Network::new()?;
	fs::write(args.outdir.join("network.dot"), network.to_dot())?;

	let transformed_network = ConnectionsToNode::traverse(&network);
	fs::write(
		args.outdir.join("transformed.dot"),
		traformed_to_dot(&transformed_network, &network.lines, &network.stations),
	)?;

	// DfsBruteForce::traverse(&network);

	Ok(())
}

struct ConnectionsToNode<'a> {
	graph: &'a Graph<StationQid, LineQid>,

	output_map: HashMap<EdgeIndex, NodeIndex>,
	output: &'a mut Graph<LineQid, StationQid>,
}

impl ConnectionsToNode<'_> {
	fn traverse(network: &Network) -> Graph<LineQid, StationQid> {
		let mut output = Graph::new();

		ConnectionsToNode {
			graph: &network.graph,

			output_map: HashMap::new(),
			output: &mut output,
		}
		.continue_at();

		output
	}

	fn continue_at(&mut self) {
		for edge_idx in self.graph.edge_indices() {
			let new_edge = *self
				.output_map
				.entry(edge_idx)
				.or_insert_with(|| self.output.add_node(self.graph[edge_idx]));

			let (_src, target) = self.graph.edge_endpoints(edge_idx).unwrap();
			for target_edge in self.graph.edges(target) {
				let new_target_edge = *self
					.output_map
					.entry(target_edge.id())
					.or_insert_with(|| self.output.add_node(self.graph[target_edge.id()]));

				self.output
					.add_edge(new_edge, new_target_edge, self.graph[target]);
			}
		}
	}
}
