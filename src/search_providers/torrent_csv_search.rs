use crate::search_providers::SearchProvider;
use crate::torrent::Torrent;

use async_trait::async_trait;
use hyper::{body::HttpBody, Body, Client, Request};
use hyper_tls::HttpsConnector;
use log::info;
use serde::Deserialize;

use std::error::Error;

#[derive(Debug, Deserialize)]
pub struct Entry {
    pub infohash: String,
    pub name: String,
    pub size_bytes: Option<u32>,
    pub created_unix: Option<u32>,
    pub seeders: Option<u32>,
    pub leechers: Option<u32>,
    pub completed: Option<u32>,
    pub scraped_date: Option<i32>,
}

pub struct TorrentCsvSearch {
    connection: Client<HttpsConnector<hyper::client::HttpConnector>>,
}

impl TorrentCsvSearch {
    pub fn new() -> TorrentCsvSearch {
        let https = HttpsConnector::new();
        let client = hyper::Client::builder().build::<_, hyper::Body>(https);
        TorrentCsvSearch { connection: client }
    }
}

#[async_trait]
impl SearchProvider for TorrentCsvSearch {
    async fn search(&self, term: &str) -> Result<Vec<Torrent>, Box<dyn Error + Send + Sync>> {
        info!("Searching on Torrent-CSV");
        let url = format!("https://torrents-csv.ml/service/search?size=300&q={}", term);

        let request = Request::get(url)
            .header(hyper::header::USER_AGENT, super::USER_AGENT)
            .body(Body::empty())
            .expect("Request builder");

        let mut res = self.connection.request(request).await?;

        info!("Status: {}", res.status());
        let mut bytes = Vec::new();
        while let Some(next) = res.data().await {
            let chunk = next?;
            bytes.extend(chunk);
        }

        let body = String::from_utf8(bytes)?;
        parse_torrent_csv(&body)
    }

    fn get_name(&self) -> &'static str {
        "TC"
    }
}

fn parse_torrent_csv(content: &str) -> Result<Vec<Torrent>, Box<dyn Error + Send + Sync>> {
    let entries: Vec<Entry> = serde_json::from_str(content)?;

    let results = entries
        .iter()
        .map(|entry| Torrent {
            name: entry.name.clone(),
            magnet_link: format!("magnet:?xt=urn:btih:{}", entry.infohash),
            seeders: entry.seeders,
            leechers: entry.leechers,
        })
        .collect();
    Ok(results)
}

#[cfg(test)]
mod test {
    static TEST_DATA: &str = include_str!("test_data/torrent-csv.json");
    static TEST_DATA_EMPTY: &str = include_str!("test_data/torrent-csv-empty.json");

    #[test]
    fn test_parse_torrent_csv() {
        let torrents = super::parse_torrent_csv(TEST_DATA).unwrap();
        assert_eq!(torrents.len(), 8);
        for torrent in torrents.iter() {
            assert!(torrent.magnet_link.starts_with("magnet:?"));
            assert!(torrent.seeders.is_some());
            assert!(torrent.leechers.is_some());
        }
    }

    #[test]
    fn test_parse_torrent_csv_empty() {
        let torrents = super::parse_torrent_csv(TEST_DATA_EMPTY).unwrap();
        assert_eq!(torrents.len(), 0);
    }
}
