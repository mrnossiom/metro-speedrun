use std::{
	fs,
	path::{Path, PathBuf},
};

use clap::{Parser, ValueEnum};
use eyre::Context;

mod glpk;
mod livesplit;
mod network;
mod queries;
mod scip;

use crate::{
	network::{Arc, InvertedNetwork, Network},
	queries::SubwayData,
};

#[derive(clap::Parser)]
struct Args {
	/// City whose subway we want to speedrun
	city: City,

	/// Directory used to store intermediary artefacts
	#[clap(long, default_value = ".cache")]
	cache: PathBuf,

	/// Directory where artefacts are written to
	#[clap(long, default_value = "output")]
	output: PathBuf,

	/// Reads the given solution and outputs a colored graph with the path taken
	#[clap(long)]
	solution: Option<PathBuf>,

	#[clap(long)]
	run: Option<RunKind>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum RunKind {
	Scip,
	Glpk,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum City {
	Paris,
	Berlin,
	Marseille,
	Toulouse,
	London,
	Nyc,
	Tokyo,
}

impl City {
	fn has_verified_data(&self) -> bool {
		matches!(self, Self::Paris)
	}

	fn qid(&self) -> &'static str {
		match self {
			Self::Paris => "Q50716",
			Self::Berlin => "Q68646",
			Self::Marseille => "Q275267",
			Self::Toulouse => "Q1129485",
			Self::London => "Q20075",
			Self::Nyc => "Q7733",
			Self::Tokyo => "Q962135",
		}
	}
}

fn main() -> eyre::Result<()> {
	let Args {
		city,
		// not respected
		cache: _,
		output,
		solution,
		run,
	} = Args::parse();

	if !city.has_verified_data() {
		eprintln!("warn: selected city data has not been verified and may not work properly.")
	}

	let data = SubwayData::fetch(city.qid()).wrap_err("could not fetch subway data")?;

	let path = output.join("network.dot");
	let network = Network::from(&data);

	if let Some(solution) = solution {
		let solution = read_solution(&solution)?;
		let path = output.join("network.solution.dot");
		println!("saving solution to `{}`", path.display());
		fs::write(
			path,
			network.to_dot_with_selected_arcs_colored(&data, &solution.iter().cloned().collect()),
		)?;

		let lss = livesplit::make_splitfile(&data, &solution);
		let path = output.join("solution.lss");
		println!("saving livesplit run to `{}`", path.display());
		fs::write(path, serde_xml_rs::to_string(&lss).unwrap())?;

		return Ok(());
	}

	// only clean output dir when redoing a computation
	if fs::exists(&output)? {
		fs::remove_dir_all(&output)?;
	}
	fs::create_dir_all(&output)?;

	println!("saving network to `{}`", path.display());
	fs::write(path, network.to_dot(&data))?;

	let mut stripped_network = network.clone();
	let nb_stripped = stripped_network.strip_line_ends();
	println!("stripped {} irrelevant nodes from the network", nb_stripped);

	let path = output.join("network.stripped.dot");
	println!("saving stripped network to `{}`", path.display());
	fs::write(path, stripped_network.to_dot(&data))?;

	let path = output.join("inverted.dot");
	println!("saving inverted network to `{}`", path.display());
	let inv_network = InvertedNetwork::from(&stripped_network);
	fs::write(path, inv_network.to_dot(&data))?;

	match run {
		None => {}
		Some(RunKind::Scip) => {
			let path = output.join("scip");
			fs::create_dir(&path)?;
			scip::PaperOpt::new(path, &stripped_network, &data).run()?;
		}
		Some(RunKind::Glpk) => {
			glpk::PaperOpt::invoke(&stripped_network, &data)?;
		}
	}

	Ok(())
}

fn read_solution(path: &Path) -> eyre::Result<Vec<Arc>> {
	let mut csv = csv::Reader::from_path(path).wrap_err("could not read given solution")?;
	let headers = csv.headers().wrap_err("could not parse CSV header")?;

	// TODO: change for a more elegant error, ensures that no line is interpreted as a header
	if headers.as_slice() != "sourcetargetline" {
		eyre::bail!("header does not match expected `source,target,line`")
	}

	let mut path = Vec::new();
	for arc in csv.deserialize::<Arc>() {
		let arc = arc?;
		path.push(arc);
	}
	Ok(path)
}
