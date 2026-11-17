use std::{collections::BTreeMap, fmt, fs, path::PathBuf};

use eyre::{ContextCompat, bail};
use petgraph::visit::EdgeRef;
use russcip::{ProblemCreated, Variable, prelude::*};

use crate::{
	network::Network,
	queries::{
		SubwayData,
		types::{LineQid, StationQid},
	},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Station {
	Real(StationQid),
	Source,
	Target,
}

impl fmt::Display for Station {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Real(id) => id.fmt(f),
			Self::Source => write!(f, "source"),
			Self::Target => write!(f, "target"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Line {
	Real(LineQid),
	Dummy,
}

impl fmt::Display for Line {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Real(id) => id.fmt(f),
			Self::Dummy => write!(f, "dummy"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Arc(Station, Station, Line);

pub struct PaperOpt {
	output: PathBuf,
	model: Option<Model<ProblemCreated>>,
	rev_follows: BTreeMap<String, (Arc, Variable)>,
}

impl PaperOpt {
	pub fn new(output: PathBuf, network: &Network, data: &SubwayData) -> Self {
		let mut mdl = Model::new()
			.include_default_plugins()
			// Limit memory to 10GiB
			.set_memory_limit(10 * 1024)
			.create_prob("metro shortest path");

		// linear binary variable is more performant than just a binary one,
		// though is it not always a correct transformation

		let real_stations = data.stations.keys();
		let stations = real_stations
			.map(|id| Station::Real(*id))
			.chain([Station::Source, Station::Target])
			.collect::<Vec<_>>();

		let real_lines = data.lines.iter();
		let lines = real_lines
			.map(|line| Line::Real(line.qid))
			.chain([Line::Dummy])
			.collect::<Vec<_>>();

		let real_arcs = network.graph.edge_references().map(|edge| {
			Arc(
				Station::Real(network.graph[edge.source()]),
				Station::Real(network.graph[edge.target()]),
				Line::Real(data.lines[edge.weight().index()].qid),
			)
		});
		let source_arcs = network
			.graph
			.node_weights()
			.map(|station| Arc(Station::Source, Station::Real(*station), Line::Dummy));
		let target_arcs = network
			.graph
			.node_weights()
			.map(|station| Arc(Station::Real(*station), Station::Target, Line::Dummy));
		let arcs = real_arcs
			.chain(source_arcs)
			.chain(target_arcs)
			.collect::<Vec<_>>();

		let follows = arcs
			.iter()
			.map(|arc| {
				let Arc(source, target, line) = arc;
				let var_name = format!("follow_{}_{}_{}", source, target, line);
				let binary_var = var()
					.bin()
					// TODO: can we use linear binary variables instead?
					// .cont(0.0..=1.0)
					.name(&var_name)
					// we want to minimize the number of follows
					.obj(1.0);
				(arc, mdl.add(binary_var))
			})
			.collect::<BTreeMap<_, _>>();
		let rev_follows = follows
			.iter()
			.map(|(k, v)| (v.name(), (**k, v.clone())))
			.collect::<BTreeMap<_, _>>();

		let flows = arcs
			.iter()
			.map(|arc| {
				let Arc(source, target, line) = arc;
				let var_name = format!("flow_{}_{}_{}", source, target, line);
				let linear_natural_var = var().cont(0.0..).name(&var_name);
				(arc, mdl.add(linear_natural_var))
			})
			.collect::<BTreeMap<_, _>>();

		let ys = stations
			.iter()
			.map(|qid| {
				let var_name = format!("y_{}", qid);
				let linear_natural_var = var().cont(0.0..);
				(qid, mdl.add(linear_natural_var.name(&var_name)))
			})
			.collect::<BTreeMap<_, _>>();

		// #1 Odd degree
		for station in stations.iter().filter(|s| matches!(s, Station::Real(_))) {
			let mut c = cons().eq(0.0);
			for arc in follows.keys().filter(|Arc(_, dst, _)| dst == station) {
				c = c.coef(&follows[arc], 1.0);
			}
			for arc in follows.keys().filter(|Arc(src, _, _)| src == station) {
				c = c.coef(&follows[arc], -1.0);
			}
			mdl.add(c);
		}
		// #2 Source/Target Expect Degree
		let mut c = cons().eq(1.0);
		for var in arcs
			.iter()
			.filter(|Arc(src, _, _)| matches!(src, Station::Source))
		{
			c = c.coef(&follows[var], 1.0);
		}
		mdl.add(c);
		let mut c = cons().eq(1.0);
		for var in arcs
			.iter()
			.filter(|Arc(_, dst, _)| matches!(dst, Station::Target))
		{
			c = c.coef(&follows[var], 1.0);
		}
		mdl.add(c);

		// #3 Visit All Lines
		for current_line in lines {
			let mut c = cons().ge(1.0);
			for arc in arcs.iter().filter(|Arc(_, _, line)| *line == current_line) {
				c = c.coef(&follows[arc], 1.0)
			}
			mdl.add(c);
		}

		// #4 Flow capacity
		for arc in &arcs {
			let mut c = cons().ge(0.0);
			c = c.coef(&follows[arc], stations.len() as f64);
			c = c.coef(&flows[arc], -1.0);
			mdl.add(c);
		}

		// #5 Flow linearity
		for station in stations.iter().filter(|s| !matches!(s, Station::Source)) {
			let mut c = cons().ge(0.0);
			for arc in arcs.iter().filter(|Arc(_, dst, _)| dst == station) {
				c = c.coef(&flows[arc], 1.0);
			}
			for arc in arcs.iter().filter(|Arc(src, _, _)| src == station) {
				c = c.coef(&flows[arc], -1.0);
			}
			c = c.coef(&ys[station], -1.0);
			mdl.add(c);
		}

		// #6 Flow connectivity
		for station in &stations {
			let mut c = cons().ge(0.0);
			c = c.coef(&ys[station], 1.0);
			for arc in arcs.iter().filter(|Arc(_, dst, _)| dst == station) {
				c = c.coef(&follows[arc], -1.0);
			}
			for arc in arcs.iter().filter(|Arc(src, _, _)| src == station) {
				c = c.coef(&follows[arc], -1.0);
			}
			mdl.add(c);
		}

		// Minimize variables with respect to their objective coef
		mdl = mdl.minimize();

		mdl = mdl.show_output();

		Self {
			output,
			model: Some(mdl),
			rev_follows,
		}
	}

	pub fn run(&mut self) -> eyre::Result<()> {
		let mdl = self.model.take().unwrap();

		// TODO: add ability to preload a solution to resume a previous run
		// mdl.add_sol(solution).unwrap();

		let format = "cip";
		let binding = self.output.join("scipmodel").with_extension(format);
		mdl.write(binding.as_os_str().to_str().unwrap(), format, true)
			.unwrap();

		let mdl = mdl.solve_concurrent();

		let Some(solutions) = mdl.get_sols() else {
			bail!("no solution found");
		};

		for (idx, solution) in solutions.iter().enumerate() {
			let objective = solution.obj_val();
			let nb_transitions = objective as u32 - 2;

			eprintln!("found solution {idx} with {nb_transitions} transitions",);

			let name_map: BTreeMap<String, f64> = solution.as_name_map().into_iter().collect();

			fs::write(
				self.output.join(format!("solution{idx:0>3}.dat")),
				dump_solution_raw(&name_map),
			)?;
			fs::write(
				self.output.join(format!("solution{idx:0>3}.txt")),
				dump_solution_path(&self.rev_follows, &name_map)?,
			)?;
		}

		Ok(())
	}
}

fn dump_solution_raw(name_map: &BTreeMap<String, f64>) -> String {
	std::fmt::from_fn(|f| {
		for (name, value) in name_map {
			writeln!(f, "{name},{value}")?;
		}
		Ok(())
	})
	.to_string()
}

fn dump_solution_path(
	rfollows: &BTreeMap<String, (Arc, Variable)>,
	name_map: &BTreeMap<String, f64>,
) -> eyre::Result<String> {
	let mut arcs = name_map
		.keys()
		.flat_map(|var| rfollows.get(var.strip_prefix("t_").unwrap()))
		.collect::<Vec<_>>();

	let mut path = Vec::new();

	// order path
	let mut current_src = Station::Source;
	loop {
		let idx = arcs
			.iter()
			// TODO: handle paths that go A -> B -> C -> B -> D, be careful not to skip the roundtrip
			.position(|(Arc(src, _, _), _)| *src == current_src)
			.wrap_err("could not transform to path, solution is malformed")?;

		let (Arc(src, dst, line), _) = arcs.swap_remove(idx);

		match (src, dst, line) {
			(Station::Real(src), Station::Real(dst), Line::Real(line)) => {
				path.push((*src, *dst, *line));
			}
			(_, Station::Target, _) => {
				assert!(arcs.is_empty(), "there are some arcs left {arcs:#?}");
				break;
			}
			_ => {}
		}

		current_src = *dst
	}

	let display = std::fmt::from_fn(|f| {
		writeln!(f, "source,target,line")?;
		for (source, target, line) in &path {
			writeln!(f, "{source},{target},{line}")?;
		}
		Ok(())
	});

	Ok(display.to_string())
}
