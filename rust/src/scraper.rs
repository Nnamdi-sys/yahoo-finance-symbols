use std::{path::Path, sync::Arc};
use std::error::Error;
use rusqlite::params;
use scraper::{Html, Selector};
use r2d2::Pool;
use rusqlite::{Connection, Result};
use r2d2_sqlite::SqliteConnectionManager;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::sync::Semaphore;


#[derive(Debug, Serialize, Deserialize)]
struct SymbolList {
    pub symbols: Vec<Ticker>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Ticker {
    pub symbol: String,
    pub name: String,
    pub category: String,
    pub asset_class: String,
    pub exchange: String,
}

pub async fn save_symbols(db_path: &Path) -> Result<(), Box<dyn Error>> {
    let manager = SqliteConnectionManager::file(db_path);
    let pool = Pool::new(manager)?;
    let conn = pool.get()?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS symbols (
             symbol TEXT PRIMARY KEY,
             name TEXT,
             category TEXT,
             asset_class TEXT,
             exchange TEXT
         )",
        [],
    )?;

    let base_url = "https://finance.yahoo.com/lookup/";
    let search_set: Vec<String> = (b'A'..=b'Z')
        .chain(b'0'..=b'9')
        .map(|c| format!("{}", c as char))
        .chain(
            (b'A'..=b'Z')
                .flat_map(|c1| (b'A'..=b'Z').map(move |c2| format!("{}{}", c1 as char, c2 as char))),
        )
        .collect();
    let total_steps = search_set.len();

    // Create and configure the progress bar
    let pb = ProgressBar::new(total_steps as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")?
            .progress_chars("#>-"),
    );

    let concurrency_limit = 10; // Set the desired concurrency limit
    let semaphore = Arc::new(Semaphore::new(concurrency_limit));
    let mut tasks = Vec::new();

    for symbol in search_set {
        let pool = pool.clone();
        let pb = pb.clone();
        let semaphore = semaphore.clone();
        pb.set_message("Scraping Symbols from Yahoo Finance");

        let task = tokio::task::spawn(async move {
            let _permit = semaphore.acquire().await.expect("Semaphore acquire failed");

            match scrape_symbols(base_url, "all", &symbol).await {
                Ok(result) => {
                    let conn = pool.get().expect("Failed to get connection from pool");
                    for doc in result {
                        if !document_exists_in_db(&conn, &doc) {
                            insert_document(&conn, &doc).unwrap();
                        }
                    }
                }
                Err(e) => eprintln!("Error scraping symbols: {:?}", e),
            }

            pb.inc(1);
        });

        tasks.push(task);
    }

    join_all(tasks).await;
    pb.finish_with_message("Completed symbol scraping");

    Ok(())
}


async fn scrape_symbols(base_url: &str, sector: &str, symbol: &str) -> Result<Vec<Ticker>, Box<dyn Error>> {
    let url =   format!("{}{}?s={}&t=A&b=0&c=5000", base_url, sector, symbol);
    let client = Client::new();
    let response = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .send()
        .await?;
    if response.status().as_u16() != 200{
        eprintln!("Bad Response: {:?}", &response)
    }
    let body = response.text().await?;

    let document = Html::parse_document(&body);
    let mut result: Vec<Ticker> = Vec::new();

    // Selector for the table rows containing symbol data
    let row_selector = Selector::parse("table tbody tr").unwrap();

    // Extract symbol data
    for row in document.select(&row_selector) {
        let mut columns: Vec<String> = Vec::new();

        // Extract data from each cell in the row
        for cell in row.select(&Selector::parse("td").unwrap()) {
            columns.push(cell.inner_html().trim().to_string());
        }

        if columns.len() >= 6 {
            let symbol_struct = Ticker {
                symbol: {
                    let symbol_html = &columns[0];
                    let symbol_document = Html::parse_fragment(&symbol_html);
                    symbol_document
                        .select(&Selector::parse("a").unwrap())
                        .next()
                        .map(|a| a.value().attr("data-symbol").unwrap_or_default())
                        .unwrap_or_default()
                        .to_string()
                },
                name: columns[1].clone(),
                category: {
                    let category_html = &columns[3];
                    let category_document = Html::parse_fragment(&category_html);
                    category_document
                        .select(&Selector::parse("a").unwrap())
                        .next()
                        .map(|a| a.inner_html().trim().to_string())
                        .unwrap_or("N/A".to_string())
                },
                asset_class: columns[4].clone(),
                exchange: columns[5].clone(),
            };

            result.push(symbol_struct);
        }
    }
    Ok(result)
}

fn document_exists_in_db(conn: &Connection, doc: &Ticker) -> bool {
    let sql = "SELECT COUNT(*) FROM symbols WHERE symbol = ?";
    let count: i64 = conn.query_row(sql, &[&doc.symbol], |row| row.get(0)).unwrap_or(0);

    count > 0
}

fn insert_document(conn: &Connection, doc: &Ticker) -> Result<()> {
    let sql = "INSERT INTO symbols (symbol, name, category, asset_class, exchange) VALUES (?, ?, ?, ?, ?)";
    conn.execute(
        sql,
        params![
            &doc.symbol,
            html_escape::decode_html_entities(&doc.name).to_string(),
            &doc.category,
            &doc.asset_class,
            &doc.exchange
        ],
    )?;
    Ok(())
}
