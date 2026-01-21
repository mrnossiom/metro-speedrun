use std::{collections::BTreeMap, fs};

use petgraph::visit::EdgeRef;
use russcip::{Solution, prelude::*};

use crate::{
	network::{Arc, Network},
	queries::SubwayData,
};

struct SolutionHandler {}

impl Eventhdlr for SolutionHandler {
	fn get_type(&self) -> EventMask {
		EventMask::SOL_FOUND | EventMask::BEST_SOL_FOUND
	}

	fn execute(&mut self, model: Model<Solving>, eventhdlr: SCIPEventhdlr, event: Event) {
		match event.event_type() {
			EventMask::SOL_FOUND => {
				println!("sol");
			}
			EventMask::BEST_SOL_FOUND => {
				println!("best sol");
			}
			_ => {}
		}
	}
}

struct MultipleSolutions {}

impl Conshdlr for MultipleSolutions {
	fn check(
		&mut self,
		model: Model<Solving>,
		conshdlr: SCIPConshdlr,
		solution: &Solution,
	) -> bool {
		true
		// false
	}

	fn enforce(&mut self, model: Model<Solving>, conshdlr: SCIPConshdlr) -> ConshdlrResult {
		ConshdlrResult::SolveLP
	}
}

pub struct PaperOpt {}

impl PaperOpt {
	pub fn invoke(network: &Network, data: &SubwayData) -> Result<(), Box<dyn std::error::Error>> {
		let mut mdl = Model::new()
			.include_default_plugins()
			// Limit memory to 10GiB
			.set_memory_limit(10 * 1024)
			.create_prob("metro shortest path");

		let stations = data.stations.keys().collect::<Vec<_>>();
		let lines = data.lines.iter().map(|line| line.qid).collect::<Vec<_>>();
		let arcs = network
			.graph
			.edge_references()
			.map(|edge| {
				(
					network.graph[edge.source()],
					network.graph[edge.target()],
					data.lines[edge.weight().index()].qid,
				)
			})
			.collect::<Vec<_>>();

		let follows = arcs
			.iter()
			.map(|arc| {
				let (source, target, line) = &arc;
				let var_name = format!("follow_{}_{}_{}", source, target, line);
				let binary_var = var()
					.bin()
					.name(&var_name)
					// we want to minimize the number of follows
					.obj(1.0);
				(arc, mdl.add(binary_var))
			})
			.collect::<BTreeMap<_, _>>();
		let rfollows = follows
			.iter()
			.map(|(k, v)| (i32::try_from(v.index()).unwrap(), *k))
			.collect::<BTreeMap<_, _>>();
		let source_follows = stations
			.iter()
			.map(|station| {
				let source_name = format!("follow_source_{}_dummy", station);
				let linear_binary_var = var().cont(0.0..=1.0).name(&source_name);
				(station, mdl.add(linear_binary_var))
			})
			.collect::<BTreeMap<_, _>>();
		let target_follows = stations
			.iter()
			.map(|station| {
				let target_name = format!("follow_{}_target_dummy", station);
				let linear_binary_var = var().cont(0.0..=1.0).name(&target_name);
				(station, mdl.add(linear_binary_var))
			})
			.collect::<BTreeMap<_, _>>();

		let flows = arcs
			.iter()
			.map(|arc| {
				let (source, target, line) = &arc;
				let var_name = format!("flow_{}_{}_{}", source, target, line);
				let linear_natural_var = var().cont(0.0..).name(&var_name);
				(arc, mdl.add(linear_natural_var))
			})
			.collect::<BTreeMap<_, _>>();
		let source_flows = stations
			.iter()
			.map(|station| {
				let source_name = format!("flow_source_{}_dummy", station);
				let linear_natural_var = var().cont(0.0..).name(&source_name);
				(station, mdl.add(linear_natural_var))
			})
			.collect::<BTreeMap<_, _>>();
		let target_flows = stations
			.iter()
			.map(|station| {
				let target_name = format!("flow_{}_target_dummy", station);
				let linear_natural_var = var().cont(0.0..).name(&target_name);
				(station, mdl.add(linear_natural_var))
			})
			.collect::<BTreeMap<_, _>>();

		let ys = stations
			.iter()
			.map(|qid| (qid, mdl.add(var().name(&format!("y_{}", qid.id())).bin())))
			.collect::<BTreeMap<_, _>>();

		// #1 Odd degree
		for station in data.stations.keys() {
			let mut c = cons().eq(0.0);
			for (source, target, line) in follows.keys() {
				if target == station {
					c = c.coef(&follows[&(*source, *target, *line)], 1.0);
				}
			}
			for (source, target, line) in follows.keys() {
				if source == station {
					c = c.coef(&follows[&(*source, *target, *line)], -1.0);
				}
			}
			mdl.add(c);
		}

		// #2 Source/Target Expect Degree
		let mut c = cons().eq(1.0);
		for var in source_follows.values() {
			c = c.coef(var, 1.0);
		}
		mdl.add(c);

		let mut c = cons().eq(1.0);
		for var in target_follows.values() {
			c = c.coef(var, 1.0);
		}
		mdl.add(c);

		// #3 Visit All Lines
		for current_line in lines {
			let mut c = cons().ge(1.0);
			for ((_source, _target, line), var) in &follows {
				if current_line == *line {
					c = c.coef(var, 1.0)
				}
			}
			mdl.add(c);
		}

		// #4 Flow capacity
		for arc in &arcs {
			let mut c = cons().ge(0.0);
			c = c.coef(&follows[&arc], (stations.len() + 2) as f64);
			c = c.coef(&flows[&arc], -1.0);
			mdl.add(c);
		}

		// #5 Flow linearity
		for station in &stations {
			let mut c = cons().ge(0.0);
			for arc in arcs.iter().filter(|(_, dst, _)| dst == *station) {
				c = c.coef(&flows[arc], 1.0);
			}
			for arc in arcs.iter().filter(|(src, _, _)| src == *station) {
				c = c.coef(&flows[arc], -1.0);
			}
			c = c.coef(&ys[station], -1.0);
			mdl.add(c);
		}

		// #6 Flow connectivity
		for station in &stations {
			let mut c = cons().ge(0.0);
			c = c.coef(&ys[station], 1.0);
			for arc in arcs.iter().filter(|(_, dst, _)| dst == *station) {
				c = c.coef(&follows[arc], -1.0);
			}
			for arc in arcs.iter().filter(|(src, _, _)| src == *station) {
				c = c.coef(&follows[arc], -1.0);
			}
			mdl.add(c);
		}

		// Minimize variables with respect to their objective coef
		mdl = mdl.minimize();

		let cons_handler = MultipleSolutions {};
		mdl.include_conshdlr("", "", -1, -1, Box::new(cons_handler));
		let sol_handler = SolutionHandler {};
		mdl.include_eventhdlr("", "", Box::new(sol_handler));

		mdl = mdl.show_output();

		let mdl = mdl.solve();

		let solution = mdl.best_sol().unwrap();

		let path = solution
			.as_id_map()
			.keys()
			// ignore dummy follows
			.flat_map(|var| rfollows.get(var))
			.cloned()
			.cloned()
			.collect::<Vec<_>>();

		fs::write("foo", format_solution(&path))?;

		Ok(())
	}
}

fn format_solution(arcs: &[Arc]) -> String {
	std::fmt::from_fn(|f| {
		writeln!(f, "source,target,line")?;
		for (source, target, line) in arcs {
			writeln!(f, "{source},{target},{line}")?;
		}
		Ok(())
	})
	.to_string()
}
