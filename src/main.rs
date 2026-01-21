use std::{fs, path::PathBuf};

use clap::Parser;

mod glpk;
mod network;
mod queries;
mod scip;
mod traversals;

use crate::{
	network::{InvertedNetwork, Network},
	queries::SubwayData,
};

#[derive(clap::Parser)]
struct Args {
	#[clap(default_value = ".cache")]
	cache_dir: PathBuf,

	#[clap(default_value = "output")]
	output_dir: PathBuf,
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

	fs::create_dir_all(&args.output_dir)?;

	let data = SubwayData::fetch(qid::PARIS_SUBWAY)?;

	let network = Network::from(&data);
	fs::write(args.output_dir.join("network.dot"), network.to_dot(&data))?;

	let mut stripped_network = network;
	let nb_stripped = stripped_network.strip_line_ends();
	println!("stripped {} irrelevant nodes from the network", nb_stripped);

	fs::write(
		args.output_dir.join("network.stripped.dot"),
		stripped_network.to_dot(&data),
	)?;

	let inv_network = InvertedNetwork::from(&stripped_network);
	fs::write(
		args.output_dir.join("inverted.dot"),
		inv_network.to_dot(&data),
	)?;

	// scip::PaperOpt::invoke(&stripped_network, &data)?;
	// glpk::PaperOpt::invoke(&stripped_network, &data)?;
	// traversals::NormalDfs::traverse(&stripped_network, &data);
	// traversals::InvertedBfs::traverse(&inv_network, &data);

	Ok(())
}
