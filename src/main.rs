use std::{fs, path::PathBuf};

use clap::Parser;

mod network;
mod queries;
mod traversals;

use crate::{
	network::{InvertedNetwork, Network},
	queries::SubwayData,
};

#[derive(clap::Parser)]
struct Args {
	#[clap(default_value = "output")]
	outdir: PathBuf,
}

const PARIS_SUBWAY_QID: &str = "Q50716";
const TOULOUSE_SUBWAY_QID: &str = "Q1129485";
const MARSEILLE_SUBWAY_QID: &str = "Q275267";
const LONDON_SUBWAY_QID: &str = "Q20075";
const TOKYO_SUBWAY_QID: &str = "Q962135";
const NYC_SUBWAY_QID: &str = "Q7733";
const BERLIN_SUBWAY_QID: &str = "Q68646";

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args = Args::parse();

	fs::create_dir_all(&args.outdir)?;

	let data = SubwayData::fetch(PARIS_SUBWAY_QID)?;

	let mut network = Network::from(&data);
	fs::write(args.outdir.join("network.dot"), network.to_dot(&data))?;

	network.strip_line_ends();
	fs::write(
		args.outdir.join("network.stripped.dot"),
		network.to_dot(&data),
	)?;

	let inv_network = InvertedNetwork::from(&network);
	fs::write(args.outdir.join("inverted.dot"), inv_network.to_dot(&data))?;

	// traversals::NormalDfs::traverse(&network, &data);
	traversals::InvertedBfs::traverse(&inv_network, &data);

	Ok(())
}
