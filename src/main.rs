use std::{collections::HashMap, fs, path::PathBuf};

use clap::Parser;
use petgraph::{
	Graph,
	dot::{Config, Dot},
	graph::{EdgeReference, NodeIndex, UnGraph},
	visit::EdgeRef,
};

mod queries;

use crate::queries::fetch_query;

#[derive(clap::Parser)]
struct Args {
	output: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args = Args::parse();

	let lines_query = fetch_query::<queries::Line>(include_str!("../queries/lines.sparql"))?;
	let stations_query =
		fetch_query::<queries::Station>(include_str!("../queries/stations.sparql"))?;
	let connections_query =
		fetch_query::<queries::Connection>(include_str!("../queries/connections.sparql"))?;

	let mut lines = HashMap::<String, queries::Line>::new();
	let mut stations = HashMap::<String, NodeIndex>::new();
	let mut graph = UnGraph::<queries::Station, String>::new_undirected();

	// Collect all lines
	for line in lines_query {
		let line = line?;
		lines.insert(line.id.clone(), line);
	}

	// Collect all stations
	for station in stations_query {
		let station = station?;

		let station_id = station.id.clone();
		let idx = graph.add_node(station);
		stations.insert(station_id, idx);
	}

	// Link stations
	for connection in connections_query {
		let connection = connection?;

		let station = *stations.get(&connection.station_id).unwrap();
		let adjacent = *stations.get(&connection.adjacent_station_id).unwrap();

		if graph
			.edges(adjacent)
			.any(|edge| edge.target() == station && *edge.weight() == connection.line_id)
		{
			continue;
		}

		graph.add_edge(station, adjacent, connection.line_id.clone());
	}

	let get_edge_attrs = |_graph, edge: EdgeReference<'_, String>| {
		format!(
			r##"color = "#{}" penwidth=2.0"##,
			lines.get(edge.weight()).unwrap().color
		)
	};
	let get_node_attrs = |_graph, (_idx, station): (NodeIndex, &queries::Station)| {
		format!(r#"pos = "{}!""#, station.coords)
	};
	let dot = Dot::with_attr_getters(
		&graph,
		&[Config::EdgeNoLabel],
		&get_edge_attrs,
		&get_node_attrs,
	);
	fs::write(args.output, format!("{}", dot))?;

	Ok(())
}
