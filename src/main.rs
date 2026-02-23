use std::{
	collections::BTreeSet,
	fs,
	path::{Path, PathBuf},
};

use clap::{Parser, ValueEnum};
use eyre::Context;

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
	/// The WikiData Qid of the city whose subway we want to speedrun
	city_qid: String,

	/// Used to store intermediary artefacts
	#[clap(long, default_value = ".cache")]
	cache_dir: PathBuf,

	/// Most artefacts are printed in there
	#[clap(long, default_value = "output")]
	output_dir: PathBuf,

	/// Reads the given solution and outputs a colored graph with the path taken
	#[clap(long)]
	solution_path: Option<PathBuf>,

	#[clap(long)]
	run: Option<RunKind>,
}

#[derive(Debug, Clone, ValueEnum)]
enum RunKind {
	Scip,
	Glpk,
}

mod qid {
	#![allow(dead_code)]
	pub const BERLIN_SUBWAY: &str = "Q68646";
	pub const MARSEILLE_SUBWAY: &str = "Q275267";
	pub const PARIS_SUBWAY: &str = "Q50716";
	pub const TOULOUSE_SUBWAY: &str = "Q1129485";

	// not working yet
	pub const LONDON_SUBWAY: &str = "Q20075";
	pub const NYC_SUBWAY: &str = "Q7733";
	pub const TOKYO_SUBWAY: &str = "Q962135";
}

fn main() -> eyre::Result<()> {
	let args = Args::parse();

	let data = SubwayData::fetch(&args.city_qid).wrap_err("could not fetch subway data")?;

	let path = args.output_dir.join("network.dot");
	let network = Network::from(&data);

	if let Some(solution_path) = args.solution_path {
		let solution = read_solution(&solution_path)?;
		let path = args.output_dir.join("network.solution.dot");
		println!("saving solution to `{}`", path.display());
		fs::write(
			path,
			network.to_dot_with_selected_arcs_colored(&data, &solution),
		)?;
		return Ok(());
	}

	// only clean output dir when redoing a computation
	if fs::exists(&args.output_dir)? {
		fs::remove_dir_all(&args.output_dir)?;
	}
	fs::create_dir_all(&args.output_dir)?;

	println!("saving network to `{}`", path.display());
	fs::write(path, network.to_dot(&data))?;

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

	match args.run {
		None => {}
		Some(RunKind::Scip) => {
			let path = args.output_dir.join("scip");
			fs::create_dir(&path)?;
			scip::PaperOpt::new(path, &stripped_network, &data).run()?;
		}
		Some(RunKind::Glpk) => {
			glpk::PaperOpt::invoke(&stripped_network, &data)?;
		}
	}

	// traversals::NormalDfs::traverse(&stripped_network, &data);
	// traversals::InvertedBfs::traverse(&inv_network, &data);

	Ok(())
}

fn read_solution(path: &Path) -> eyre::Result<BTreeSet<Arc>> {
	let mut csv = csv::Reader::from_path(path).wrap_err("could not read given solution")?;
	let headers = csv.headers().wrap_err("could not parse CSV header")?;

	// TODO: change for a more elegant error, ensures that no line is interpreted as a header
	if headers.as_slice() != "sourcetargetline" {
		eyre::bail!("header does not match expected `source,target,line`")
	}

	let mut set = BTreeSet::new();
	for arc in csv.deserialize::<Arc>() {
		let arc = arc?;
		set.insert(arc);
	}
	Ok(set)
}
