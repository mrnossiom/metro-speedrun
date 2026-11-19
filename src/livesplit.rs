use serde::{Deserialize, Serialize};

use crate::{network::Arc, queries::SubwayData};

pub(crate) fn make_splitfile(data: &SubwayData, path: &[Arc]) -> Run {
	let make_station = |qid| Segment {
		name: data.stations[qid].name.clone(),
		..Default::default()
	};
	let make_line = |qid| Segment {
		name: data.lines[data.line_map[qid].index()].name.clone(),
		..Default::default()
	};

	let mut prev_line = None;
	let mut segments = Vec::new();
	let mut backup = None;
	for (_, station, line) in path.iter() {
		if prev_line != Some(line)
			&& let Some((station, line)) = backup
		{
			segments.push(make_line(line));
			segments.push(make_station(station));
		}
		backup = Some((station, line));
		prev_line = Some(line);
	}
	if let Some((station, line)) = backup {
		segments.push(make_line(line));
		segments.push(make_station(station));
	}
	segments.pop();

	Run {
		version: "1.8.1".into(),
		game_name: "Métro de Paris".into(),
		category_name: "Any%".into(),
		offset: "0".into(),
		segments: Segments { segment: segments },
		..Default::default()
	}
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct Run {
	#[serde(rename = "@version")]
	pub version: String,
	pub game_icon: String,
	pub game_name: String,
	pub category_name: String,
	pub offset: String,
	pub attempt_count: i64,
	pub segments: Segments,
	pub segment_groups: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct Segments {
	pub segment: Vec<Segment>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct Segment {
	pub name: String,
	pub icon: String,
}
