use std::{
	collections::{BTreeMap, HashMap},
	fmt,
};

use petgraph::{
	Graph,
	dot::{Config, Dot},
	graph::{EdgeReference, NodeIndex},
	visit::EdgeRef,
};

use crate::queries::{LineId, SubwayData, types::StationQid};

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

impl Network {
	/// Remove every station that is useless in our optimized subway traversal,
	/// i.e. stations that are on line ends
	pub fn strip_line_ends(&mut self) -> u32 {
		// We could do a better O(n) algorithm with a stable graph

		let mut nb_stripped = 0;

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
