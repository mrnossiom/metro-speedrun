use csv::DeserializeRecordsIntoIter;
use reqwest::blocking::Client;
use serde::{Deserialize, de::DeserializeOwned};
use std::{collections::HashMap, fmt, fs, io::Write, path::Path};

const CACHE_DIR: &str = ".cache/queries";

pub struct SubwayData {
	pub lines: HashMap<types::LineQid, Line>,
	pub stations: HashMap<types::StationQid, Station>,

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

		let mut lines = HashMap::new();
		let mut stations = HashMap::new();
		let mut connections = Vec::new();

		// Collect all lines
		for line in lines_query {
			let line = line?;
			lines.insert(line.id, line);
		}

		// Collect all stations
		for station in stations_query {
			let station = station?;
			stations.insert(station.id, station);
		}

		// Link stations
		for connection in connections_query {
			let connection = connection?;
			connections.push(connection);
		}

		Ok(Self {
			lines,
			stations,
			connections,
		})
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
	pub id: types::LineQid,
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
	pub id: types::StationQid,
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
	pub station_id: types::StationQid,
	pub adjacent_station_id: types::StationQid,
	pub line_id: types::LineQid,
}

pub mod types {
	use std::fmt;

	use serde::{
		Deserialize, Deserializer,
		de::{self, Visitor},
	};

	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
	pub struct Qid(pub u32);

	impl fmt::Display for Qid {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			write!(f, "Q{}", self.0)
		}
	}

	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
	pub struct LineQid(pub Qid);

	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
	pub struct StationQid(pub Qid);

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
