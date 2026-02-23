use std::{fs, path::Path, process::Command};

use csv::DeserializeRecordsIntoIter;
use eyre::{Context, bail};
use serde::de::DeserializeOwned;

use crate::{
	network::{Arc, Network},
	queries::SubwayData,
};

const CACHE_DIR: &str = ".cache/glpsol";

pub struct PaperOpt {}

impl PaperOpt {
	pub fn invoke(network: &Network, data: &SubwayData) -> eyre::Result<()> {
		let model = include_str!("../metro.mod");

		let station_qids = data
			.stations
			.keys()
			.map(|station_qid| station_qid.to_string())
			.collect::<Vec<_>>();
		let line_qids = data
			.lines
			.iter()
			.map(|line| line.qid.to_string())
			.collect::<Vec<_>>();
		let arcs = network
			.graph
			.raw_edges()
			.iter()
			.map(|edge| {
				format!(
					"({},{},{})",
					network.graph[edge.source()],
					network.graph[edge.target()],
					data.lines[edge.weight.index()].qid,
				)
			})
			.collect::<Vec<_>>();
		let model_data = format!(
			"\
set RealStations := \n{}\n;
set RealLines := \n{}\n;
set RealArcs := \n{}\n;
end;
",
			station_qids.join("\n"),
			line_qids.join("\n"),
			arcs.join("\n"),
		);

		let result = invoke_optimizer::<Arc>("metro", model, &model_data)?;

		println!();
		for arcs_path in result.into_iter() {
			let (source, target, line) = arcs_path?;
			println!(
				"{} → {} | {}",
				&data.stations[&source].name,
				&data.stations[&target].name,
				&data.lines[data.line_map[&line].index()]
			);
		}

		Ok(())
	}
}

fn invoke_optimizer<T>(
	name: &str,
	model: &str,
	data: &str,
) -> eyre::Result<DeserializeRecordsIntoIter<std::fs::File, T>>
where
	T: DeserializeOwned,
{
	fs::create_dir_all(CACHE_DIR)?;

	let mut hasher = blake3::Hasher::new();
	hasher.update(model.as_bytes());
	hasher.update(data.as_bytes());
	let mut fingerprint = hasher.finalize().to_hex();
	fingerprint.truncate(8);
	let cache_file = format!("{CACHE_DIR}/{name}_{fingerprint}.txt");

	if !Path::new(&cache_file).exists() {
		println!("computing `{name}` solution to {cache_file}");

		let glpk_dir = format!("/tmp/glpsol-{name}-{fingerprint}");
		let glpk_dir = Path::new(&glpk_dir);
		let glpk_model = glpk_dir.join("metro.mod");
		let glpk_data = glpk_dir.join("metro.dat");
		let glpk_output = glpk_dir.join("output.txt");

		fs::create_dir_all(glpk_dir)?;

		fs::write(&glpk_model, model)?;
		fs::write(&glpk_data, data)?;

		let mut cmd = Command::new("glpsol");
		cmd.arg("--model").arg(&glpk_model);
		cmd.arg("--data").arg(&glpk_data);
		cmd.arg("--output").arg(&glpk_output);
		cmd.arg("--display").arg(&cache_file);

		if !cmd.status().wrap_err("could not run glpsol")?.success() {
			bail!("could not compute gmpl model")
		}

		fs::remove_dir_all(glpk_dir)?;
	} else {
		println!("solution `{name}` cached in {cache_file}");
	}

	let csv = csv::Reader::from_path(cache_file)?;

	Ok(csv.into_deserialize())
}
