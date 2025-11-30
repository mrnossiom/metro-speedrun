use std::collections::{HashMap, HashSet};

use petgraph::{Graph, graph::NodeIndex, visit::EdgeRef};

use crate::{
	network::Network,
	queries::{
		self, SubwayData,
		types::{LineQid, Qid, StationQid},
	},
};

#[derive(Debug)]
pub struct DfsBruteForce<'a> {
	graph: &'a Graph<StationQid, LineQid>,

	lines: &'a HashMap<LineQid, queries::Line>,
	stations: &'a HashMap<StationQid, queries::Station>,

	start_station: StationQid,
	route: Vec<(LineQid, StationQid)>,

	lines_taken: HashSet<LineQid>,
	line_streak: Streak<LineQid>,
}

impl DfsBruteForce<'_> {
	pub fn traverse(network: &Network, data: &SubwayData) {
		let mut traversal = DfsBruteForce {
			graph: &network.graph,

			lines: &data.lines,
			stations: &data.stations,

			start_station: StationQid(Qid(0)),
			route: Vec::new(),

			lines_taken: HashSet::new(),
			line_streak: Streak::new(LineQid(Qid(0))),
		};
		for node_idx in network.graph.node_indices() {
			println!(
				"starting from node {}",
				data.stations[&network.graph[node_idx]]
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
