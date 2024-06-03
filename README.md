# YAHOO FINANCE SYMBOLS

This Library helps in scraping over 200,000+ symbols from yahoo finance. The symbols are saved in a local sqlite database which can be used directly or accessed with the rust or python library functions.

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

all_symbols = ys.get_symbols()
print(all_symbols)

symbols = ys.search_symbols("Bitcoin", "ETF")
print(symbols)
```

### Rust

``` rust
use yahoo_finance_symbols::keys::{AssetClass, Category, Exchange};
use yahoo_finance_symbols::{get_symbols, search_symbols};

let all_symbols = get_symbols(AssetClass::All, Category::All, Exchange::All)?;
println!("{:?}", all_symbols);

let symbols = search_symbols("Apple", "Equity").unwrap();
println!("{:?}", symbols);
```

