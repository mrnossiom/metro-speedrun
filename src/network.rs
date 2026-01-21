use std::{
	collections::{BTreeMap, BTreeSet, HashMap},
	fmt,
	sync::Mutex,
};

use petgraph::{
	Direction, Graph,
	dot::{Config, Dot},
	graph::{EdgeReference, NodeIndex},
	visit::EdgeRef,
};

use crate::queries::{
	LineId, SubwayData,
	types::{LineQid, StationQid},
};

#[derive(Clone)]
pub struct Network {
	pub graph: Graph<StationQid, LineId>,
}

impl From<&SubwayData> for Network {
	fn from(data: &SubwayData) -> Self {
		let SubwayData {
			lines: _,
			line_map,
			stations,
			connections,
		} = &data;

		let mut station_nodes = BTreeMap::new();
		let mut graph = Graph::new();

		// Collect all stations
		for station_qid in stations.keys() {
			let idx = graph.add_node(*station_qid);
			station_nodes.insert(*station_qid, idx);
		}

		// Link stations
		for connection in connections {
			let station = station_nodes[&connection.station_qid];
			let adjacent = station_nodes[&connection.adjacent_station_qid];

			graph.add_edge(station, adjacent, line_map[&connection.line_qid]);
		}

		Self { graph }
	}
}

pub type Arc = (StationQid, StationQid, LineQid);

impl Network {
	/// Remove every station that is irrelevant to the subway problem,
	/// i.e. stations that are on line ends
	pub fn strip_line_ends(&mut self) -> u32 {
		// We could do a better O(n) algorithm with a stable graph

		let mut nb_stripped = 0;

		'main: loop {
			for node_idx in self.graph.node_indices() {
				let in_edges = self
					.graph
					.edges_directed(node_idx, Direction::Incoming)
					.map(|edge| {
						let (neighbor, _) = self.graph.edge_endpoints(edge.id()).unwrap();
						let line = self.graph[edge.id()];
						(neighbor, line)
					});
				let out_edges = self
					.graph
					.edges_directed(node_idx, Direction::Outgoing)
					.map(|edge| {
						let (_, neighbor) = self.graph.edge_endpoints(edge.id()).unwrap();
						let line = self.graph[edge.id()];
						(neighbor, line)
					});

				#[derive(Debug)]
				enum LineNeighbors {
					None,
					One { neighbor: NodeIndex, line: LineId },
					Many,
				}
				let neighbors =
					in_edges
						.chain(out_edges)
						.fold(LineNeighbors::None, |n, (neighbor, line)| match n {
							LineNeighbors::None => LineNeighbors::One { neighbor, line },
							current @ LineNeighbors::One {
								neighbor: cur_neighbor,
								line: cur_line,
							} if cur_neighbor == neighbor && cur_line == line => current,
							_ => LineNeighbors::Many,
						});

				if let LineNeighbors::None = &neighbors {
					self.graph.remove_node(node_idx);
					nb_stripped += 1;
					continue 'main;
				} else if let LineNeighbors::One { neighbor: target, line } = neighbors
					// and this one neighbor is only connected to the same line
					&& self.graph.edges_directed(target, Direction::Outgoing)
						.chain(self.graph.edges_directed(target, Direction::Incoming))
						.all(|nline| *nline.weight() == line)
				{
					self.graph.remove_node(node_idx);
					nb_stripped += 1;
					continue 'main;
				}
			}
			break 'main;
		}

		nb_stripped
	}

	pub fn to_dot(&self, data: &SubwayData) -> String {
		struct NetworkDisplay<'a> {
			net: &'a Network,
			data: &'a SubwayData,
		}

		impl fmt::Display for NetworkDisplay<'_> {
			fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
				let get_node_attrs = |_graph, (_idx, station_id): (NodeIndex, &StationQid)| {
					format!(
						r#"pos = "{}!" fontsize = 12.0"#,
						self.data.stations[station_id].coords,
					)
				};
				let get_edge_attrs = |graph: &Graph<StationQid, LineId>,
				                      edge: EdgeReference<'_, LineId>| {
					let (src, dst) = graph.edge_endpoints(edge.id()).unwrap();
					// eprintln!("{:?} {:?} {:?}", edge.weight(), graph[src], graph[dst]);
					format!(
						r##"color = "#{}" penwidth = 2.0"##,
						self.data.lines[edge.weight().index()].color
					)
				};
				let dot = Dot::with_attr_getters(
					&self.net.graph,
					&[Config::EdgeNoLabel],
					&get_edge_attrs,
					&get_node_attrs,
				);

				dot.graph_fmt(
					f,
					|station_id, f| f.write_str(&self.data.stations[station_id].name),
					|_line, _f| Ok(()),
				)
			}
		}

		NetworkDisplay { net: self, data }.to_string()
	}

	pub fn to_dot_arcs(&self, data: &SubwayData, arcs: &BTreeSet<Arc>) -> String {
		struct NetworkDisplay<'a> {
			net: &'a Network,
			data: &'a SubwayData,
			arcs: &'a BTreeSet<Arc>,
		}

		impl fmt::Display for NetworkDisplay<'_> {
			fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
				let get_node_attrs = |_graph, (_idx, station_id): (NodeIndex, &StationQid)| {
					format!(
						r#"pos = "{}!" fontsize = 12.0"#,
						self.data.stations[station_id].coords,
					)
				};
				let get_edge_attrs = |graph: &Graph<StationQid, LineId>,
				                      edge: EdgeReference<'_, LineId>| {
					let (src, dst) = graph.edge_endpoints(edge.id()).unwrap();
					// eprintln!("{:?} {:?} {:?}", edge.weight(), graph[src], graph[dst]);
					format!(
						r##"color = "#{}" penwidth = 2.0"##,
						if self.arcs.contains(&(
							self.net.graph[src],
							self.net.graph[dst],
							self.data.lines[edge.weight().index()].qid
						)) {
							&self.data.lines[edge.weight().index()].color
						} else {
							"ccc"
						}
					)
				};
				let dot = Dot::with_attr_getters(
					&self.net.graph,
					&[Config::EdgeNoLabel],
					&get_edge_attrs,
					&get_node_attrs,
				);

				dot.graph_fmt(
					f,
					|station_id, f| f.write_str(&self.data.stations[station_id].name),
					|_line, _f| Ok(()),
				)
			}
		}

		NetworkDisplay {
			net: self,
			data,
			arcs,
		}
		.to_string()
	}
}

pub struct InvertedNetwork {
	pub graph: Graph<LineId, StationQid>,
}

impl From<&Network> for InvertedNetwork {
	fn from(net: &Network) -> Self {
		let mut graph = Graph::new();

		let mut transition_nodes = HashMap::<([NodeIndex; 2], _), NodeIndex>::new();

		for edge_idx in net.graph.edge_indices() {
			let line_qid = net.graph[edge_idx];
			let (src, target) = net.graph.edge_endpoints(edge_idx).unwrap();
			let mut idx = ([src, target], line_qid);
			idx.0.sort();

			let transition_node = *transition_nodes
				.entry(idx)
				.or_insert_with(|| graph.add_node(line_qid));

			for target_edge in net.graph.edges(target) {
				let target_line_qid = net.graph[target_edge.id()];
				let (target_src, target_target) =
					net.graph.edge_endpoints(target_edge.id()).unwrap();
				let mut target_idx = ([target_src, target_target], target_line_qid);
				target_idx.0.sort();

				let target_transition_node = *transition_nodes
					.entry(target_idx)
					.or_insert_with(|| graph.add_node(target_line_qid));

				graph.add_edge(transition_node, target_transition_node, net.graph[target]);
			}
		}

		Self { graph }
	}
}

impl InvertedNetwork {
	pub fn to_dot(&self, data: &SubwayData) -> String {
		struct NetworkDisplay<'a> {
			inv_net: &'a InvertedNetwork,
			data: &'a SubwayData,
		}

		impl fmt::Display for NetworkDisplay<'_> {
			fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
				let get_node_attrs = |_graph, (_id, line_id): (NodeIndex, &LineId)| {
					format!(
						r##"style = "filled" fillcolor = "#{}" fontsize = 12.0 tooltip = "{}""##,
						self.data.lines[line_id.index()].color,
						self.data.lines[line_id.index()].name,
					)
				};
				let get_edge_attrs = |_graph, edge: EdgeReference<'_, StationQid>| {
					format!(
						r##"fontsize = 12.0 tooltip = "{}""##,
						self.data.stations[edge.weight()].name,
					)
				};
				let dot = Dot::with_attr_getters(
					&self.inv_net.graph,
					&[Config::NodeNoLabel, Config::EdgeNoLabel],
					&get_edge_attrs,
					&get_node_attrs,
				);

				dot.graph_fmt(
					f,
					|line_id, f| f.write_str(&self.data.lines[line_id.index()].name),
					|station_id, f| f.write_str(&self.data.stations[station_id].name),
				)
			}
		}

		NetworkDisplay {
			inv_net: self,
			data,
		}
		.to_string()
	}
}
