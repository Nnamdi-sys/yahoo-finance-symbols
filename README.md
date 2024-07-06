# YAHOO FINANCE SYMBOLS

This Library helps in scraping 350,000+ symbols from yahoo finance. The symbols are saved in a local sqlite database which can be used directly or accessed with the rust or python library functions.

## Installation

### Python

``` bash
pip install yahoo_finance_symbols
```

### Rust

``` bash
cargo install yahoo_finance_symbols
```


## Examples

### Python

``` python
import yahoo_finance_symbols as ys

# Fetch All Symbols
all_symbols = ys.get_symbols()
print(all_symbols)

# Search for Symbols With a Keyword
symbols = ys.search_symbols("Bitcoin", "ETF")
print(symbols)

# Update the Database
ys.update_database()
```

### Rust

``` rust
use yahoo_finance_symbols::keys::{AssetClass, Category, Exchange};
use yahoo_finance_symbols::{get_symbols, search_symbols, update_database};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    // Fetch All Symbols
    let all_symbols = get_symbols(AssetClass::All, Category::All, Exchange::All).await?;
    println!("{:?}", all_symbols);

    // Search for Symbols with a Keyword
    let symbols = search_symbols("Apple", "Equity").await?;
    println!("{:?}", symbols);

    // Update the Database
    update_database().await()?;
}
```

