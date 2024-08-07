pub mod keys;
pub mod scraper;

use r2d2::Pool;
use std::error::Error;
use std::path::PathBuf;
use polars::prelude::*;
use scraper::{download_file, save_symbols};
use std::collections::HashMap;
use rusqlite::{Result, ToSql};
use serde::{Deserialize, Serialize};
use r2d2_sqlite::SqliteConnectionManager;
use keys::{AssetClass, Category, Exchange};
use tokio::sync::OnceCell;


static DATABASE_POOL: OnceCell<Pool<SqliteConnectionManager>> = OnceCell::const_new();

async fn initialize_database() -> Result<Pool<SqliteConnectionManager>> {
    let db_file = "symbols.db";
    let db_path = PathBuf::from(db_file);

    if !db_path.exists() {
        let url = "https://github.com/Nnamdi-sys/yahoo-finance-symbols/raw/main/rust/src/symbols.db";
        if download_file(url, &db_path).await.is_err() {
            println!("Unable to download database from: {}. Scraping symbols now from Yahoo Finance", url);
            save_symbols(&db_path).await.expect("Failed to Get Symbols Database");
        }
    }

    let manager = SqliteConnectionManager::file(db_file);
    let pool = Pool::new(manager).expect("Failed to create database connection pool");

    Ok(pool)
}

async fn get_database_pool() -> Result<&'static Pool<SqliteConnectionManager>> {
    if DATABASE_POOL.get().is_none() {
        let pool = initialize_database().await?;
        DATABASE_POOL.set(pool).unwrap();
    }
    Ok(DATABASE_POOL.get().unwrap())
}



pub async fn update_database() -> Result<(), Box<dyn Error>> {
    let db_file = "symbols.db";
    let db_path = PathBuf::from(db_file);

    if db_path.exists() {
        tokio::fs::remove_file(&db_path).await?;
    }

    save_symbols(&db_path).await?;

    println!("Database updated successfully.");

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub symbol: String,
    pub name: String,
    pub category: String,
    pub asset_class: String,
    pub exchange: String,
}


impl Symbol {
    pub fn new() -> Symbol {
        Symbol {
            symbol: String::new(),
            name: String::new(),
            category: String::new(),
            asset_class: String::new(),
            exchange: String::new(),
        }
    }
}

/// Fetches a symbol from the database
///
/// # Arguments
///
/// * `symbol` - Symbol string
///
/// # Returns
///
/// * `Symbol` - Symbol struct
///
/// # Example
///
/// ```
/// use std::error::Error;
/// use yahoo_finance_symbols::get_symbol;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn Error>> {
///     let result = get_symbol("AAPL").await?;
///     println!("{:?}", result);
///     Ok(())
/// }
/// ```
pub async fn get_symbol(symbol: &str) -> Result<Symbol> {
    let pool = get_database_pool().await?;
    let conn = pool.get().expect("Failed to get connection from pool");
    let mut stmt = conn.prepare("SELECT * FROM symbols WHERE symbol = ?")
        .expect("Failed to prepare statement");

    let symbol_row = stmt.query_row(&[symbol], |row| {
        Ok(Symbol {
            symbol: row.get(0)?,
            name: row.get(1)?,
            category: row.get(2)?,
            asset_class: row.get(3)?,
            exchange: row.get(4)?,
        })
    });

    symbol_row
}

/// Fetches symbols that match the specified asset class, category, and exchange from the database
///
/// # Arguments
///
/// * `asset_class` - Asset class enum
/// * `category` - Category enum
/// * `exchange` - Exchange enum
///
/// # Returns
///
/// * `Vec<Symbol>` - Vector of symbols
///
/// # Example
///
/// ```
/// use std::error::Error;
/// use yahoo_finance_symbols::keys::{AssetClass, Category, Exchange};
/// use yahoo_finance_symbols::get_symbols;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn Error>> {
///     let result = get_symbols(AssetClass::Stocks, Category::Technology, Exchange::NASDAQ).await?;
///     println!("{:?}", result);
///     let result = get_symbols(AssetClass::ETFs, Category::All, Exchange::All).await?;
///     println!("{:?}", result);
///     let result = get_symbols(AssetClass::Futures, Category::All, Exchange::All).await?;
///     println!("{:?}", result);
///     let result = get_symbols(AssetClass::Indices, Category::All, Exchange::All).await?;
///     println!("{:?}", result);
///     let result = get_symbols(AssetClass::MutualFunds, Category::All, Exchange::All).await?;
///     println!("{:?}", result);
///     let result = get_symbols(AssetClass::Cryptocurrencies, Category::All, Exchange::All).await?;
///     println!("{:?}", result);
///     let result = get_symbols(AssetClass::Currencies, Category::All, Exchange::All).await?;
///     println!("{:?}", result);
///     Ok(())
/// }
/// ```
pub async fn get_symbols(asset_class: AssetClass, category: Category, exchange: Exchange) -> Result<Vec<Symbol>> {
    let pool = get_database_pool().await?;
    let conn = pool.get().expect("Failed to get connection from pool");

    // Prepare a dynamic number of placeholders and values based on the provided filters
    let (mut placeholders, mut values): (Vec<String>, Vec<&dyn ToSql>) = (Vec::new(), Vec::new());

    let asset_classes = asset_class.to_string_vec().await;
    let categories = category.to_string_vec().await;
    let exchanges = exchange.to_string_vec().await;

    placeholders.push(format!("asset_class IN ({})", (0..asset_classes.len()).map(|_| "?").collect::<Vec<_>>().join(",")));
    values.extend(asset_classes.iter().map(|s| s as &dyn ToSql));

    placeholders.push(format!("category IN ({})", (0..categories.len()).map(|_| "?").collect::<Vec<_>>().join(",")));
    values.extend(categories.iter().map(|s| s as &dyn ToSql));

    placeholders.push(format!("exchange IN ({})", (0..exchanges.len()).map(|_| "?").collect::<Vec<_>>().join(",")));
    values.extend(exchanges.iter().map(|s| s as &dyn ToSql));

    let query = format!("SELECT * FROM symbols WHERE {}", placeholders.join(" AND "));

    let mut stmt = conn.prepare(&query).expect("Failed to prepare statement");

    let rows = stmt.query_map(&*values, |row| {
        Ok(Symbol {
            symbol: row.get(0)?,
            name: row.get(1)?,
            category: row.get(2)?,
            asset_class: row.get(3)?,
            exchange: row.get(4)?,
        })
    })?;

    let symbols: Result<Vec<Symbol>> = rows.collect();
    symbols
}

pub async fn get_symbols_count() -> Result<i64> {
    let pool = get_database_pool().await?;
    let conn = pool.get().expect("Failed to get connection from pool");
    let sql = "SELECT COUNT(*) FROM symbols";
    let count: i64 = conn.query_row(sql, [], |row| row.get(0))?;
    Ok(count)
}

pub async fn get_distinct_exchanges() -> Result<Vec<String>> {
    let pool = get_database_pool().await?;
    let conn = pool.get().expect("Failed to get connection from pool");
    let mut stmt = conn
        .prepare("SELECT DISTINCT exchange FROM symbols")
        .expect("Failed to prepare statement");

    let rows = stmt.query_map([], |row| {
        Ok( row.get(0)? )
    })?;

    let exchanges: Result<Vec<String>> = rows.collect();
    exchanges
}

pub async fn get_distinct_categories() -> Result<Vec<String>> {
    let pool = get_database_pool().await?;
    let conn = pool.get().expect("Failed to get connection from pool");
    let mut stmt = conn
        .prepare("SELECT DISTINCT category FROM symbols")
        .expect("Failed to prepare statement");

    let rows = stmt.query_map([], |row| {
        Ok( row.get(0)? )
    })?;

    let categories: Result<Vec<String>> = rows.collect();
    categories
}

pub async fn get_distinct_asset_classes() -> Result<Vec<String>> {
    let pool = get_database_pool().await?;
    let conn = pool.get().expect("Failed to get connection from pool");
    let mut stmt = conn
        .prepare("SELECT DISTINCT asset_class FROM symbols")
        .expect("Failed to prepare statement");

    let rows = stmt.query_map([], |row| {
        Ok( row.get(0)? )
    })?;

    let asset_classes: Result<Vec<String>> = rows.collect();
    asset_classes
}


/// Fetches ticker symbols that closely match the specified query and asset class
///
/// # Arguments
///
/// * `query` - ticker symbol query
/// * `asset_class` - asset class (Equity, ETF, Mutual Fund, Index, Currency, Futures, Crypto)
///
/// # Returns
///
/// * `HashMap<String, String>` - dictionary of ticker symbols and names
///
/// # Example
///
/// ```
/// use yahoo_finance_symbols::search_symbols;
/// use std::error::Error;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn Error>> {
///     let symbols = search_symbols("Apple", "Equity").await?;
///     println!("{:?}", symbols);
///     Ok(())
/// }
/// ```
pub async fn search_symbols(query: &str, asset_class: &str) -> Result<HashMap<String, String>> {
    let asset_class = match asset_class {
        "Equity" => AssetClass::Stocks,
        "ETF" => AssetClass::ETFs,
        "Mutual Fund" => AssetClass::MutualFunds,
        "Index" => AssetClass::Indices,
        "Currency" => AssetClass::Currencies,
        "Futures" => AssetClass::Futures,
        "Crypto" => AssetClass::Cryptocurrencies,
        _ => panic!("Asset class must be one of: Equity, ETF, Mutual Fund, Index, Currency, Futures, Crypto"),
    };
    let tickers = get_symbols(asset_class, Category::All, Exchange::All).await.unwrap();
    let symbols = tickers
        .iter()
        .filter(|tc| tc.symbol.to_lowercase().contains(&query.to_lowercase())
            || tc.name.to_lowercase().contains(&query.to_lowercase()))
        .map(|tc| (tc.symbol.clone(), tc.name.clone()))
        .collect::<HashMap<String, String>>();
    Ok(symbols)
}

/// Fetches all Symbols into a Polars DataFrame
/// 
/// # Returns
/// 
/// * `DataFrame` - Polars DataFrame of all Yahoo Finance Symbols
/// 
/// # Example
/// 
/// ```
/// use yahoo_finance_symbols::get_symbols_df;
/// use std::error::Error;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn Error>> {
///     let symbols_df = get_symbols_df().await?;
///     println!("{:?}", symbols_df);
///     Ok(())
/// }
/// ```
pub async fn get_symbols_df() -> Result<DataFrame, Box<dyn Error>> {
    let symbols = get_symbols(AssetClass::All, Category::All, Exchange::All).await?;

    let symbols_series: Vec<Series> = vec![
        Series::new("symbol", symbols.iter().map(|s| s.symbol.as_str()).collect::<Vec<&str>>()),
        Series::new("name", symbols.iter().map(|s| s.name.as_str()).collect::<Vec<&str>>()),
        Series::new("category", symbols.iter().map(|s| s.category.as_str()).collect::<Vec<&str>>()),
        Series::new("asset_class", symbols.iter().map(|s| s.asset_class.as_str()).collect::<Vec<&str>>()),
        Series::new("exchange", symbols.iter().map(|s| s.exchange.as_str()).collect::<Vec<&str>>()),
    ];

    let symbols_df = DataFrame::new(symbols_series)?;

    Ok(symbols_df)
}


#[cfg(test)]

mod tests {

    use crate::{get_symbols_count, get_symbols_df};

    #[tokio::test]
    async fn check_symbols_count() {
        let symbols_count = get_symbols_count().await.unwrap();
        println!("{}", symbols_count);

        let symbols_df = get_symbols_df().await.unwrap();
        println!("{:?}", symbols_df);

        assert!(symbols_count > 450_000);
    }
}


