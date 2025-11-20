use std::collections::HashMap;

use petgraph::{
	Graph,
	dot::{Config, Dot},
	graph::{EdgeReference, NodeIndex},
	visit::EdgeRef,
};

use crate::queries::{self, fetch_query, types::Qid};

pub struct Network {
	pub lines: HashMap<Qid, queries::Line>,
	pub stations: HashMap<Qid, queries::Station>,
	pub station_nodes: HashMap<Qid, NodeIndex>,

	/// Nodes are stations Qids
	/// Edges are connections between stations and labels with the line id
	pub graph: Graph<Qid, Qid>,
}

impl Network {
	pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
		let lines_query = fetch_query::<queries::Line>(include_str!("../queries/lines.sparql"))?;
		let stations_query =
			fetch_query::<queries::Station>(include_str!("../queries/stations.sparql"))?;
		let connections_query =
			fetch_query::<queries::Connection>(include_str!("../queries/connections.sparql"))?;

		let mut lines = HashMap::new();
		let mut stations = HashMap::new();
		let mut station_nodes = HashMap::new();
		let mut graph = Graph::new();

		// Collect all lines
		for line in lines_query {
			let line = line?;
			lines.insert(line.id, line);
		}

		// Collect all stations
		for station in stations_query {
			let station = station?;

			let idx = graph.add_node(station.id);
			station_nodes.insert(station.id, idx);
			stations.insert(station.id, station);
		}

		// Link stations
		for connection in connections_query {
			let connection = connection?;

			let station = station_nodes[&connection.station_id];
			let adjacent = station_nodes[&connection.adjacent_station_id];

			if graph
				.edges(station)
				.any(|edge| edge.target() == adjacent && *edge.weight() == connection.line_id)
			{
				continue;
			}

			graph.add_edge(station, adjacent, connection.line_id);
		}

		Ok(Self {
			lines,
			stations,
			station_nodes,
			graph,
		})
	}

	pub fn pre_process(&mut self) {
		// TODO
	}

	pub fn to_dot(&self) -> String {
		let get_node_attrs = |_graph, (_idx, station_id): (NodeIndex, &queries::types::Qid)| {
			format!(
				r#"label = "{}" pos = "{}!" fontsize = 12.0"#,
				self.stations[station_id].name, self.stations[station_id].coords
			)
		};
		let get_edge_attrs = |_graph, edge: EdgeReference<'_, Qid>| {
			format!(
				r##"color = "#{}" penwidth = 2.0"##,
				self.lines[edge.weight()].color
			)
		};
		let dot = Dot::with_attr_getters(
			&self.graph,
			&[Config::EdgeNoLabel, Config::NodeNoLabel],
			&get_edge_attrs,
			&get_node_attrs,
		);

		format!("{}", dot)
	}
}
