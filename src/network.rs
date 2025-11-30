use std::{collections::HashMap, fmt};

use petgraph::{
	Graph,
	dot::{Config, Dot},
	graph::{EdgeReference, NodeIndex},
	visit::EdgeRef,
};

use crate::queries::{
	SubwayData,
	types::{LineQid, StationQid},
};

pub struct Network {
	pub graph: Graph<StationQid, LineQid>,

	pub station_nodes: HashMap<StationQid, NodeIndex>,
}

impl From<&SubwayData> for Network {
	fn from(data: &SubwayData) -> Self {
		let SubwayData {
			stations,
			connections,
			..
		} = &data;

		let mut station_nodes = HashMap::new();
		let mut graph = Graph::new();

		// Collect all stations
		for station_id in stations.keys() {
			let idx = graph.add_node(*station_id);
			station_nodes.insert(*station_id, idx);
		}

		// Link stations
		for connection in connections {
			let station = station_nodes[&connection.station_id];
			let adjacent = station_nodes[&connection.adjacent_station_id];

			graph.add_edge(station, adjacent, connection.line_id);
		}

		Self {
			graph,
			station_nodes,
		}
	}
}

impl Network {
	/// Remove every station that is useless in our optimized subway traversal,
	/// i.e. stations that are on line ends
	pub fn strip_line_ends(&mut self) {
		// We could do a better O(n) algorithm with a stable graph

		// TODO: this suppresses the 3bis loop part that we don't care about still, the transformation is illegal
		'main: loop {
			for node_idx in self.graph.node_indices() {
				let mut neighbors = self.graph.neighbors(node_idx);
				let neighbor = neighbors.next();
				// TODO: this is not correct in theory but only affects "Chardon-Lagache"
				if neighbor.is_none() {
					self.graph.remove_node(node_idx);
					continue 'main;
				}
				if let Some(neighbor) = neighbor
					// only one neighbor
					&& neighbors.next().is_none()
					// and this neighbor connection just to one another station
					// 
					// we assume there is no subway line that is only connected to the terminus of another line
					&& self.graph.neighbors(neighbor).count() <= 2
				{
					self.graph.remove_node(node_idx);
					continue 'main;
				}
			}
			break 'main;
		}
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
						self.data.stations[station_id].coords
					)
				};
				let get_edge_attrs = |graph: &Graph<StationQid, LineQid>,
				                      edge: EdgeReference<'_, LineQid>| {
					let (src, dst) = graph.edge_endpoints(edge.id()).unwrap();
					// eprintln!("{:?} {:?} {:?}", edge.weight(), graph[src], graph[dst]);
					format!(
						r##"color = "#{}" penwidth = 2.0"##,
						self.data.lines[edge.weight()].color
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
}

pub struct InvertedNetwork {
	pub graph: Graph<LineQid, StationQid>,
}

impl From<&Network> for InvertedNetwork {
	fn from(net: &Network) -> Self {
		let mut output_map: HashMap<([NodeIndex; 2], LineQid), NodeIndex> = HashMap::new();
		let mut graph = Graph::new();

		for edge_idx in net.graph.edge_indices() {
			let line_qid = net.graph[edge_idx];
			let (src, target) = net.graph.edge_endpoints(edge_idx).unwrap();
			let mut idx = ([src, target], line_qid);
			idx.0.sort();

			let transition_node = *output_map
				.entry(idx)
				.or_insert_with(|| graph.add_node(line_qid));

			for target_edge in net.graph.edges(target) {
				let target_line_qid = net.graph[target_edge.id()];
				let (target_src, target_target) =
					net.graph.edge_endpoints(target_edge.id()).unwrap();
				let mut target_idx = ([target_src, target_target], target_line_qid);
				target_idx.0.sort();

				let target_transition_node = *output_map
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
				let get_node_attrs = |_graph, (_idx, line_id): (NodeIndex, &LineQid)| {
					format!(
						r##"color = "#{}" fontsize = 12.0"##,
						self.data.lines[line_id].color
					)
				};
				let get_edge_attrs = |_graph, _edge: EdgeReference<'_, StationQid>| {
					r##"fontsize = 12.0"##.to_string()
				};
				let dot = Dot::with_attr_getters(
					&self.inv_net.graph,
					&[],
					&get_edge_attrs,
					&get_node_attrs,
				);

				dot.graph_fmt(
					f,
					|line_id, f| f.write_str(&self.data.lines[line_id].name),
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
