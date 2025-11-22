use std::{collections::HashMap, fmt};

use petgraph::{
	Graph,
	dot::{Config, Dot},
	graph::{EdgeReference, NodeIndex},
	visit::EdgeRef,
};

use crate::queries::{
	self, fetch_query,
	types::{LineQid, StationQid},
};

pub struct Network {
	pub lines: HashMap<LineQid, queries::Line>,
	pub stations: HashMap<StationQid, queries::Station>,
	pub station_nodes: HashMap<StationQid, NodeIndex>,

	pub graph: Graph<StationQid, LineQid>,
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

	pub fn to_dot(&self) -> String {
		struct NetworkDisplay<'a>(&'a Network);

		impl fmt::Display for NetworkDisplay<'_> {
			fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
				let get_node_attrs = |_graph, (_idx, station_id): (NodeIndex, &StationQid)| {
					format!(
						r#"pos = "{}!" fontsize = 12.0"#,
						self.0.stations[station_id].coords
					)
				};
				let get_edge_attrs = |_graph, edge: EdgeReference<'_, LineQid>| {
					format!(
						r##"color = "#{}" penwidth = 2.0"##,
						self.0.lines[edge.weight()].color
					)
				};
				let dot = Dot::with_attr_getters(
					&self.0.graph,
					&[Config::EdgeNoLabel],
					&get_edge_attrs,
					&get_node_attrs,
				);

				dot.graph_fmt(
					f,
					|station_id, f| f.write_str(&self.0.stations[station_id].name),
					|_line, _f| Ok(()),
				)
			}
		}

		NetworkDisplay(self).to_string()
	}
}

pub fn traformed_to_dot(
	graph: &Graph<LineQid, StationQid>,
	lines: &HashMap<LineQid, queries::Line>,
	stations: &HashMap<StationQid, queries::Station>,
) -> String {
	struct NetworkDisplay<'a> {
		graph: &'a Graph<LineQid, StationQid>,
		lines: &'a HashMap<LineQid, queries::Line>,
		stations: &'a HashMap<StationQid, queries::Station>,
	}

	impl fmt::Display for NetworkDisplay<'_> {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			let get_node_attrs = |_graph, (_idx, line_id): (NodeIndex, &LineQid)| {
				format!(
					r##"color = "#{}" fontsize = 12.0"##,
					self.lines[line_id].color
				)
			};
			let get_edge_attrs =
				|_graph, _edge: EdgeReference<'_, StationQid>| r##"fontsize = 12.0"##.to_string();
			let dot = Dot::with_attr_getters(&self.graph, &[], &get_edge_attrs, &get_node_attrs);

			dot.graph_fmt(
				f,
				|line_id, f| f.write_str(&self.lines[line_id].name),
				|station_id, f| f.write_str(&self.stations[station_id].name),
			)
		}
	}

	NetworkDisplay {
		graph,
		lines,
		stations,
	}
	.to_string()
}
