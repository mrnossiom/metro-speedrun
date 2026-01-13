use csv::DeserializeRecordsIntoIter;
use reqwest::blocking::Client;
use serde::{Deserialize, de::DeserializeOwned};
use std::{collections::BTreeMap, fmt, fs, io::Write, path::Path};

const CACHE_DIR: &str = ".cache/queries";

#[derive(Debug)]
pub struct SubwayData {
	pub lines: Vec<Line>,
	pub line_map: BTreeMap<types::LineQid, LineId>,
	pub stations: BTreeMap<types::StationQid, Station>,

	pub connections: Vec<Connection>,
}

impl SubwayData {
	pub fn fetch(subway_qid: &str) -> Result<Self, Box<dyn std::error::Error>> {
		let lines_query = include_str!("../queries/lines.sparql").replace("{subway}", subway_qid);
		let stations_query =
			include_str!("../queries/stations.sparql").replace("{subway}", subway_qid);
		let connections_query =
			include_str!("../queries/connections.sparql").replace("{subway}", subway_qid);

		let lines_query = fetch_query::<Line>("lines", &lines_query)?;
		let stations_query = fetch_query::<Station>("stations", &stations_query)?;
		let connections_query = fetch_query::<Connection>("connections", &connections_query)?;

		let mut lines = Vec::new();
		let mut line_map = BTreeMap::new();
		let mut stations = BTreeMap::new();
		let mut connections = Vec::new();

		// Collect all lines
		for (i, line) in lines_query.enumerate() {
			let line = line?;

			let line_id = LineId::new(i);
			line_map.insert(line.qid, line_id);
			lines.push(line);
		}

		// Collect all stations
		for station in stations_query {
			let station = station?;
			stations.insert(station.qid, station);
		}

		// Link stations
		for connection in connections_query {
			let connection = connection?;
			connections.push(connection);
		}

		Ok(Self {
			lines,
			line_map,
			stations,
			connections,
		})
	}
}

impl SubwayData {
	pub fn nb_lines(&self) -> u32 {
		self.lines.len() as u32
	}
}

/// Fetches a SPARQL query result, caches it on disk.
pub fn fetch_query<T>(
	name: &str,
	query: &str,
) -> Result<DeserializeRecordsIntoIter<std::fs::File, T>, Box<dyn std::error::Error>>
where
	T: DeserializeOwned,
{
	fs::create_dir_all(CACHE_DIR)?;

	let mut hasher = blake3::Hasher::new();
	hasher.update(query.as_bytes());
	let mut fingerprint = hasher.finalize().to_hex();
	fingerprint.truncate(8);
	let cache_file = format!("{CACHE_DIR}/{name}_{fingerprint}.csv");

	if !Path::new(&cache_file).exists() {
		println!("fetching `{name}` query to {cache_file}");
		let client = Client::new();
		let res = client
			.get("https://query.wikidata.org/sparql")
			.header("User-Agent", "rust-wikidata-client/0.1")
			.query(&[("query", query)])
			.header("Accept", "text/csv")
			.send()?
			.text()?;

		let mut file = fs::File::create(&cache_file)?;
		file.write_all(res.as_bytes())?;
	} else {
		println!("query `{name}` cached in {cache_file}");
	}

	let csv = csv::Reader::from_path(&cache_file)?;

	Ok(csv.into_deserialize())
}

/// Store a line number between 0 and 63.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct LineId(u8);

impl LineId {
	fn new(index: usize) -> Self {
		assert!(index < 32, "number of lines must stay under 32");
		Self(index as u8)
	}

	pub fn bitmask(&self) -> u32 {
		1u32 << self.0
	}

	pub fn index(&self) -> usize {
		self.0 as usize
	}
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
	pub qid: types::LineQid,
	pub name: String,
	pub color: String,
}

impl fmt::Display for Line {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.name)
	}
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Station {
	pub qid: types::StationQid,
	pub name: String,
	pub coords: types::Point,
}

impl fmt::Display for Station {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.name)
	}
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
	pub station_qid: types::StationQid,
	pub adjacent_station_qid: types::StationQid,
	pub line_qid: types::LineQid,
}

pub mod types {
	use std::fmt;

	use serde::{
		Deserialize, Deserializer,
		de::{self, Visitor},
	};

	#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
	pub struct Qid(u32);

	impl Qid {
		pub fn id(&self) -> u32 {
			self.0
		}
	}

	impl fmt::Display for Qid {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			write!(f, "Q{}", self.0)
		}
	}

	#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
	pub struct LineQid(Qid);

	impl fmt::Display for LineQid {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			self.0.fmt(f)
		}
	}

	impl std::ops::Deref for LineQid {
		type Target = Qid;
		fn deref(&self) -> &Self::Target {
			&self.0
		}
	}

	#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
	pub struct StationQid(Qid);

	impl fmt::Display for StationQid {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			self.0.fmt(f)
		}
	}

	impl std::ops::Deref for StationQid {
		type Target = Qid;
		fn deref(&self) -> &Self::Target {
			&self.0
		}
	}

	impl<'de> Deserialize<'de> for Qid {
		fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>,
		{
			struct QidVisitor;

			impl<'de> Visitor<'de> for QidVisitor {
				type Value = Qid;

				fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
					formatter
						.write_str("a string formatted like http://www.wikidata.org/entity/Qxxx")
				}

				fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
				where
					E: de::Error,
				{
					let qid = value
						.strip_prefix("http://www.wikidata.org/entity/Q")
						.or_else(|| value.strip_prefix("Q"))
						.ok_or_else(|| E::custom(format!("Invalid format: {}", value)))?
						.parse::<u32>()
						.map_err(|_| E::custom("Qid is not a valid u32"))?;

					Ok(Qid(qid))
				}
			}

			deserializer.deserialize_str(QidVisitor)
		}
	}

	#[derive(Debug, PartialEq)]
	pub struct Point(pub f32, pub f32);

	impl fmt::Display for Point {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			write!(f, "{},{}", self.0, self.1)
		}
	}

	impl<'de> Deserialize<'de> for Point {
		fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>,
		{
			struct PointVisitor;

			impl<'de> Visitor<'de> for PointVisitor {
				type Value = Point;

				fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
					formatter.write_str("a string formatted like Point(x y)")
				}

				fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
				where
					E: de::Error,
				{
					let inner = value
						.strip_prefix("Point(")
						.and_then(|s| s.strip_suffix(")"))
						.ok_or_else(|| E::custom(format!("Invalid format: {}", value)))?;

					let mut parts = inner.split_whitespace();

					let x = parts
						.next()
						.ok_or_else(|| E::custom("Missing X coordinate"))?
						.parse::<f32>()
						.map_err(|_| E::custom("X is not a valid f32"))?;

					let y = parts
						.next()
						.ok_or_else(|| E::custom("Missing Y coordinate"))?
						.parse::<f32>()
						.map_err(|_| E::custom("Y is not a valid f32"))?;

					Ok(Point(x, y))
				}
			}

			deserializer.deserialize_str(PointVisitor)
		}
	}
}
