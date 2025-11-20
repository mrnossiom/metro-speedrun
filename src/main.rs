use std::{
	collections::{HashMap, HashSet},
	fs, mem,
	path::PathBuf,
	time::Duration,
};

use clap::Parser;
use petgraph::{
	Graph,
	graph::NodeIndex,
	visit::{EdgeRef, NodeRef},
};

mod network;
mod queries;

use crate::{network::Network, queries::types::Qid};

#[derive(clap::Parser)]
struct Args {
	output: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args = Args::parse();

	let mut network = Network::new()?;
	network.pre_process();
	fs::write(args.output, network.to_dot())?;

	FindShortestRoute::traverse_network(&network);

	Ok(())
}

#[derive(Debug)]
pub struct FindShortestRoute<'a> {
	graph: &'a Graph<Qid, Qid>,

	lines: &'a HashMap<Qid, queries::Line>,
	stations: &'a HashMap<Qid, queries::Station>,

	start_station: Qid,
	route: Vec<(Qid, Qid)>,

	lines_taken: HashSet<Qid>,
	line_streak: Streak<Qid>,
}

impl FindShortestRoute<'_> {
	pub fn traverse_network(network: &Network) {
		let mut traversal = FindShortestRoute {
			graph: &network.graph,

			lines: &network.lines,
			stations: &network.stations,

			start_station: Qid(0),
			route: Vec::new(),

			lines_taken: HashSet::new(),
			line_streak: Streak::new(Qid(0)),
		};
		for node_idx in network.graph.node_indices() {
			// for node_idx in [NodeIndex::new(290)] {
			println!(
				"starting from node {}",
				network.stations[&network.graph[node_idx]]
			);
			traversal.start_station = traversal.graph[node_idx];
			traversal.continue_at(node_idx);
		}
	}

	pub fn continue_at(&mut self, idx: NodeIndex) {
		// std::thread::sleep(Duration::from_millis(100));
		// self.print_route();

		const MAX_LINES: usize = 16;
		const MAX_LINE_STREAK: u32 = 4;
		const MAX_ROUTE_LENGTH: usize = 100;
		// const MAX_ROUTE_LENGTH: usize = 27;

		if self.lines_taken.len() == MAX_LINES {
			self.print_route();
			// println!();
			return;
		}

		if self.line_streak.current > MAX_LINE_STREAK {
			return;
		}

		if self.route.len() == MAX_ROUTE_LENGTH {
			return;
		}

		for edge in self.graph.edges(idx) {
			let streak_save = self.line_streak.record_kind_and_save(*edge.weight());
			self.route.push((*edge.weight(), self.graph[edge.target()]));
			let was_line_added = self.lines_taken.insert(*edge.weight());

			self.continue_at(edge.target());

			if was_line_added {
				self.lines_taken.remove(edge.weight());
			}
			self.route.pop();
			self.line_streak = streak_save;
		}
	}

	fn print_route(&self) {
		print!("{} ", self.stations[&self.start_station]);
		for (line, node) in &self.route {
			print!("-({})> {} ", &self.lines[line], &self.stations[node]);
		}
		println!();
	}
}

#[derive(Debug, Default, Clone, Copy)]
struct Streak<T: Copy + PartialEq> {
	kind: T,
	current: u32,
}

impl<T: Copy + PartialEq> Streak<T> {
	fn new(initial: T) -> Self {
		Self {
			kind: initial,
			current: 0,
		}
	}

	fn record_kind_and_save(&mut self, kind: T) -> Self {
		let save = *self;
		if self.kind == kind {
			self.current += 1;
		} else {
			self.kind = kind;
			self.current = 1;
		}
		save
	}
}
