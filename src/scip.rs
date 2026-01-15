use russcip::{Solution, prelude::*};

use crate::{network::Network, queries::SubwayData};

struct SolutionHandler {}

impl Eventhdlr for SolutionHandler {
	fn get_type(&self) -> EventMask {
		EventMask::SOL_FOUND | EventMask::BEST_SOL_FOUND
	}

	fn execute(&mut self, model: Model<Solving>, eventhdlr: SCIPEventhdlr, event: Event) {
		match event.event_type() {
			EventMask::SOL_FOUND => {}
			EventMask::BEST_SOL_FOUND => {}
			_ => {}
		}

		// event.var().unwrap().sol_val()
	}
}

struct MyConshdlr;

impl Conshdlr for MyConshdlr {
	fn check(
		&mut self,
		model: Model<Solving>,
		conshdlr: SCIPConshdlr,
		solution: &Solution,
	) -> bool {
		true
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

		let follows = network
			.graph
			.raw_edges()
			.iter()
			.map(|edge| {
				mdl.add(var().cont(0.0..).name(&format!(
					"follow_{}_{}_{}",
					network.graph[edge.source()].id(),
					network.graph[edge.target()].id(),
					data.lines[edge.weight.index()].qid.id(),
				)))
			})
			.collect::<Vec<_>>();

		let flows = network
			.graph
			.raw_edges()
			.iter()
			.map(|edge| {
				mdl.add(var().cont(0.0..).name(&format!(
					"flow_{}_{}_{}",
					network.graph[edge.source()].id(),
					network.graph[edge.target()].id(),
					data.lines[edge.weight.index()].qid.id(),
				)))
			})
			.collect::<Vec<_>>();

		let ys = data
			.stations
			.keys()
			.map(|qid| mdl.add(var().name(&format!("y_{}", qid.id())).bin()))
			.collect::<Vec<_>>();

		mdl.include_conshdlr(
			"record and search of other limits",
			"",
			-1,
			-1,
			Box::new(MyConshdlr),
		);

		mdl.include_eventhdlr(
			"solution handler",
			"handle all solutions",
			Box::new(SolutionHandler {}),
		);

		mdl.status();

		Ok(())
	}
}
