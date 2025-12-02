use std::{collections::VecDeque, mem};

use petgraph::{Graph, graph::NodeIndex, visit::EdgeRef};

use crate::{
	network::{InvertedNetwork, Network},
	queries::{LineId, SubwayData, types::StationQid},
};

#[derive(Debug)]
pub struct NormalDfs<'a> {
	graph: &'a Graph<StationQid, LineId>,

	data: &'a SubwayData,

	route: Vec<NodeIndex>,
	lines_taken: u32,
	line_streak: Streak<LineId>,
}

impl NormalDfs<'_> {
	pub fn traverse(network: &Network, data: &SubwayData) {
		let mut traversal = NormalDfs {
			graph: &network.graph,
			data,

			route: Vec::default(),
			lines_taken: u32::default(),
			line_streak: Streak::default(),
		};

		for start_node_idx in network.graph.node_indices() {
			println!(
				"node {}/{}",
				start_node_idx.index(),
				network.graph.node_count()
			);

			traversal.route.clear();
			traversal.lines_taken = u32::default();
			traversal.line_streak = Streak::default();

			traversal.route.push(start_node_idx);

			traversal.continue_at(start_node_idx);
		}
	}

	pub fn continue_at(&mut self, idx: NodeIndex) {
		const MAX_LINE_STREAK: u32 = 4;
		const MAX_ROUTE_LENGTH: usize = 27;

		if self.lines_taken.count_ones() == self.data.nb_lines() {
			self.print_route();
			return;
		}

		if self.line_streak.current > MAX_LINE_STREAK {
			return;
		}

		if self.route.len() == MAX_ROUTE_LENGTH {
			return;
		}

		for edge in self.graph.edges(idx) {
			// Do not backtrack
			if edge.target() == *self.route.last().unwrap() {
				continue;
			}

			self.route.push(edge.target());

			let new_lines_taken = self.lines_taken | edge.weight().bitmask();
			let old_lines_taken = mem::replace(&mut self.lines_taken, new_lines_taken);

			let old_streak = self.line_streak.record_kind_and_save(*edge.weight());

			self.continue_at(edge.target());

			self.line_streak = old_streak;
			self.lines_taken = old_lines_taken;
			self.route.pop();
		}
	}

	fn print_route(&self) {
		// print!("{} ", self.data.stations[&self.start_station]);
		for node in &self.route {
			print!(
				"→{:3}",
				// &self.data.lines[line_qid],
				// &self.data.stations[node]
				node.index()
			);
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

pub struct InvertedBfs {}

impl InvertedBfs {
	pub fn traverse(inv_net: &InvertedNetwork, data: &SubwayData) {
		let InvertedNetwork { graph } = &inv_net;

		let mut queue = VecDeque::new();

		let mut parents = vec![NodeIndex::default(); graph.node_count()];
		let mut lines = vec![u32::default(); graph.node_count()];

		for start_node_idx in graph.node_indices() {
			queue.clear();
			parents.fill(Default::default());
			lines.fill(Default::default());

			queue.push_back(start_node_idx);

			while let Some(node_idx) = queue.pop_front() {
				let current_lines = lines[node_idx.index()] | graph[node_idx].bitmask();

				if current_lines.count_ones() > 8 {
					println!("{:016b}", current_lines);

					let mut idx = node_idx;
					while parents[idx.index()] != start_node_idx {
						print!("{} ", idx.index());
						idx = parents[idx.index()];
					}
					println!()
				}

				for neighbor in graph.neighbors(node_idx) {
					if lines[neighbor.index()] != 0 {
						continue;
					}

					lines[neighbor.index()] = current_lines;
					parents[neighbor.index()] = node_idx;

					queue.push_back(neighbor);
				}
			}
		}
	}
}
