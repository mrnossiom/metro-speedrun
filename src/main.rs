use std::{
	collections::BTreeSet,
	fs,
	path::{Path, PathBuf},
};

use clap::Parser;

mod glpk;
mod network;
mod queries;
mod scip;
mod traversals;

use crate::{
	network::{Arc, InvertedNetwork, Network},
	queries::SubwayData,
};

#[derive(clap::Parser)]
struct Args {
	#[clap(long, default_value = ".cache")]
	cache_dir: PathBuf,

	#[clap(long, default_value = "output")]
	output_dir: PathBuf,

	#[clap(long)]
	solution_path: Option<PathBuf>,
}

mod qid {
	#![allow(dead_code)]
	pub const PARIS_SUBWAY: &str = "Q50716";
	pub const TOULOUSE_SUBWAY: &str = "Q1129485";
	pub const MARSEILLE_SUBWAY: &str = "Q275267";
	pub const LONDON_SUBWAY: &str = "Q20075";
	pub const TOKYO_SUBWAY: &str = "Q962135";
	pub const NYC_SUBWAY: &str = "Q7733";
	pub const BERLIN_SUBWAY: &str = "Q68646";
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args = Args::parse();

	fs::remove_dir_all(&args.output_dir)?;
	fs::create_dir_all(&args.output_dir)?;

	let data = SubwayData::fetch(qid::PARIS_SUBWAY)?;

	let path = args.output_dir.join("network.dot");
	let network = Network::from(&data);
	println!("saving network to `{}`", path.display());
	fs::write(path, network.to_dot(&data))?;

	if let Some(solution_path) = args.solution_path {
		let solution = read_solution(&solution_path)?;
		let path = args.output_dir.join("network.solution.dot");
		println!("saving solution to `{}`", path.display());
		fs::write(path, network.to_dot_arcs(&data, &solution))?;
		return Ok(());
	}

	let mut stripped_network = network.clone();
	let nb_stripped = stripped_network.strip_line_ends();
	println!("stripped {} irrelevant nodes from the network", nb_stripped);

	let path = args.output_dir.join("network.stripped.dot");
	println!("saving stripped network to `{}`", path.display());
	fs::write(path, stripped_network.to_dot(&data))?;

	let path = args.output_dir.join("inverted.dot");
	println!("saving inverted network to `{}`", path.display());
	let inv_network = InvertedNetwork::from(&stripped_network);
	fs::write(path, inv_network.to_dot(&data))?;

	// scip::PaperOpt::invoke(&stripped_network, &data)?;
	// glpk::PaperOpt::invoke(&stripped_network, &data)?;
	// traversals::NormalDfs::traverse(&stripped_network, &data);
	// traversals::InvertedBfs::traverse(&inv_network, &data);

	Ok(())
}

fn read_solution(path: &Path) -> Result<BTreeSet<Arc>, Box<dyn std::error::Error>> {
	let mut csv = csv::Reader::from_path(path)?;
	let headers = csv.headers()?;
	// TODO: change for a more elegant error, ensures that no line is interpreted as a header
	assert_eq!(headers.as_slice(), "sourcetargetline");
	let mut set = BTreeSet::new();
	for arc in csv.deserialize::<Arc>() {
		let arc = arc?;
		set.insert(arc);
	}
	Ok(set)
}
