use std::mem;

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
		for start_node_idx in inv_net.graph.node_indices() {
			println!(
				"node {}/{}",
				start_node_idx.index() + 1,
				inv_net.graph.node_count()
			);
			Self::iddfs(inv_net, data, start_node_idx);
		}
	}

	fn iddfs(inv_net: &InvertedNetwork, data: &SubwayData, start_node_idx: NodeIndex) {
		const MIN_DEPTH: usize = 25;
		const MAX_DEPTH: usize = 30; // A reasonable upper bound to prevent infinite execution

		for depth_limit in MIN_DEPTH..=MAX_DEPTH {
			println!("depth {}/{}", depth_limit, MAX_DEPTH);

			let mut path = Vec::with_capacity(depth_limit);
			path.push(start_node_idx);

			let lines = inv_net.graph[start_node_idx].bitmask();

			// Depth remaining is `depth_limit - 1` because the path already contains the start node.
			Self::dfs_recursive(
				inv_net,
				data,
				start_node_idx,
				&mut path,
				lines,
				depth_limit.saturating_sub(1),
			);
		}
	}

	fn dfs_recursive(
		inv_net: &InvertedNetwork,
		data: &SubwayData,
		current_node_idx: NodeIndex,
		path: &mut Vec<NodeIndex>,
		lines: u32,
		depth_remaining: usize,
	) {
		if depth_remaining == 0 {
			// We have reached the target depth for this iteration.
			if lines.count_ones() == data.nb_lines() {
				// Condition met, print the path.
				print!("Path (len {}, lines {:016b}): ", path.len(), lines);
				for node in path {
					print!("{} ", node.index());
				}
				println!();
			}
			return;
		}

		for neighbor in inv_net.graph.neighbors(current_node_idx) {
			if !path.contains(&neighbor) {
				path.push(neighbor);
				let new_lines = lines | inv_net.graph[neighbor].bitmask();

				Self::dfs_recursive(
					inv_net,
					data,
					neighbor,
					path,
					new_lines,
					depth_remaining - 1,
				);

				path.pop(); // Backtrack
			}
		}
	}
}
