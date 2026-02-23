use std::{
	collections::{BTreeMap, HashMap},
	fmt, fs,
	path::PathBuf,
};

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
	output_path: PathBuf,
	model: Option<Model<ProblemCreated>>,
	rev_follows: BTreeMap<String, (Arc, Variable)>,
}

impl PaperOpt {
	pub fn new(output_path: PathBuf, network: &Network, data: &SubwayData) -> Self {
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
			output_path,
			model: Some(mdl),
			rev_follows,
		}
	}

	pub fn run(&mut self) -> eyre::Result<()> {
		let mut mdl = self.model.take().unwrap();
		let mut solution_nb = 0;

		// TODO: add ability to preload a solution to resume a previous run
		// mdl.add_sol(solution).unwrap();

		loop {
			let solved_mdl = mdl.solve();
			let Some(solution) = solved_mdl.best_sol() else {
				bail!("stopping at solution {solution_nb}");
			};

			let objective = solution.obj_val();

			eprintln!(
				"found solution {solution_nb} with {} transitions",
				objective as u32 - 2
			);

			let name_map = solution.as_name_map();

			// SAFETY: this should not be held beyond the `free_transform` call
			drop(solution);

			fs::write(
				self.output_path
					.join(format!("solution{solution_nb:0>2}.dat")),
				dump_solution(&name_map),
			)?;
			fs::write(
				self.output_path
					.join(format!("solution{solution_nb:0>2}.txt")),
				dump_solution_path(&self.rev_follows, &name_map)?,
			)?;

			// Reset model with added constraint for the last solution and retry
			mdl = solved_mdl.free_transform();
			solution_nb += 1;
			constrain_solution_in_model(&mut mdl, &self.rev_follows, &name_map, objective);
		}
	}
}

fn dump_solution(name_map: &HashMap<String, f64>) -> String {
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
	name_map: &HashMap<String, f64>,
) -> eyre::Result<String> {
	let arcs = name_map
		.keys()
		.flat_map(|var| rfollows.get(var.strip_prefix("t_").unwrap()))
		.collect::<Vec<_>>();

	let mut path = Vec::new();

	// order path
	let mut current_src = Station::Source;
	loop {
		let (Arc(src, dst, line), _) = arcs
			.iter()
			.find(|(Arc(src, _, _), _)| *src == current_src)
			.wrap_err("could not transform to path, solution is malformed")?;

		match (src, dst, line) {
			(Station::Real(src), Station::Real(dst), Line::Real(line)) => {
				path.push((*src, *dst, *line));
			}
			(_, Station::Target, _) => break,
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

fn constrain_solution_in_model(
	mdl: &mut Model<ProblemCreated>,
	rev_follows: &BTreeMap<String, (Arc, Variable)>,
	name_map: &HashMap<String, f64>,
	objective: f64,
) {
	let mut c = cons();

	let active_follow_vars = name_map
		.iter()
		// keep only active binary variables
		// we do this instead of (== 1.0) because we use linear binary variables
		.filter(|(_, value)| **value > 0.5)
		// keep only follow variables
		.flat_map(|(id, _)| rev_follows.get(id.strip_prefix("t_").unwrap()))
		.map(|(_arc, var)| var)
		.collect::<Vec<_>>();

	for var in &active_follow_vars {
		c = c.coef(var, 1.0);
	}

	c = c.le(objective - 1.0);

	mdl.add(c);
}
