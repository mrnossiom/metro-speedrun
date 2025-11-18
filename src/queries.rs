use csv::DeserializeRecordsIntoIter;
use reqwest::blocking::Client;
use serde::{Deserialize, de::DeserializeOwned};
use std::{fmt, fs, io::Write, path::Path};

const CACHE_DIR: &str = ".cache/queries";

/// Fetches a SPARQL query result, caches it on disk.
pub fn fetch_query<T>(
	query: &str,
) -> Result<DeserializeRecordsIntoIter<std::fs::File, T>, Box<dyn std::error::Error>>
where
	T: DeserializeOwned,
{
	fs::create_dir_all(CACHE_DIR)?;

	let mut hasher = blake3::Hasher::new();
	hasher.update(query.as_bytes());
	let hash = format!("{}", hasher.finalize().to_hex());
	let cache_file = format!("{}/{}.csv", CACHE_DIR, hash);

	if !Path::new(&cache_file).exists() {
		println!("fetching query from Wikidata API");
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
		println!("reading query from cache: {}", cache_file);
	}

	let csv = csv::Reader::from_path(&cache_file)?;

	Ok(csv.into_deserialize())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
	pub id: String,
	pub name: String,
	pub color: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Station {
	pub id: String,
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
	pub station_id: String,
	pub adjacent_station_id: String,
	pub line_id: String,
}

mod types {
	use std::fmt;

	use serde::{
		Deserialize, Deserializer,
		de::{self, Visitor},
	};

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

				// This function is called when Serde encounters a string in the input
				fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
				where
					E: de::Error,
				{
					// Step A: Check format and strip wrapping "Point(" and ")"
					// We trim matches to handle strict format "Point(...)"
					let inner = value
						.strip_prefix("Point(")
						.and_then(|s| s.strip_suffix(")"))
						.ok_or_else(|| E::custom(format!("Invalid format: {}", value)))?;

					// Step B: Split by whitespace to get the two numbers
					let mut parts = inner.split_whitespace();

					// Step C: Parse the first number
					let x_str = parts
						.next()
						.ok_or_else(|| E::custom("Missing X coordinate"))?;
					let x = x_str
						.parse::<f32>()
						.map_err(|_| E::custom("X is not a valid f32"))?;

					// Step D: Parse the second number
					let y_str = parts
						.next()
						.ok_or_else(|| E::custom("Missing Y coordinate"))?;
					let y = y_str
						.parse::<f32>()
						.map_err(|_| E::custom("Y is not a valid f32"))?;

					Ok(Point(x, y))
				}
			}

			deserializer.deserialize_str(PointVisitor)
		}
	}
}
