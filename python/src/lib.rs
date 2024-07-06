use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;
use yahoo_finance_symbols::{get_symbols, update_database};
use yahoo_finance_symbols::keys::{AssetClass, Exchange, Category};


#[pymodule]
#[pyo3(name = "yahoo_finance_symbols")]
fn yahoo_finance_symbols_py(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(update_database_py, m)?).unwrap();
    m.add_function(wrap_pyfunction!(search_symbols_py, m)?).unwrap();
    m.add_function(wrap_pyfunction!(get_symbols_py, m)?).unwrap();
    Ok(())
}
   
    
#[pyfunction]
#[pyo3(name = "search_symbols")]
/// Fetches ticker symbols that closely match the specified query and asset class
///
/// # Arguments
///
/// * `query` - `str` - ticker symbol query
/// * `asset_class` - `str` - asset class (Equity, ETF, Mutual Fund, Index, Currency, Futures, Crypto)
///
/// # Returns
///
/// `dict` - dictionary of ticker symbols and names
///
/// # Example
///
/// ```
/// import yahoo_finance_symbols as ys
///
/// symbols = ys.search_symbols("Apple", "Equity")
/// print(symbols)
/// ```
pub fn search_symbols_py(query: String, asset_class: String) -> PyObject {
    let asset_class = match asset_class.as_str() {
        "Equity" => AssetClass::Stocks,
        "ETF" => AssetClass::ETFs,
        "Mutual Fund" => AssetClass::MutualFunds,
        "Index" => AssetClass::Indices,
        "Currency" => AssetClass::Currencies,
        "Futures" => AssetClass::Futures,
        "Crypto" => AssetClass::Cryptocurrencies,
        _ => panic!("Asset class must be one of: Equity, ETF, Mutual Fund, Index, Currency, Futures, Crypto"),
    };
    let tickers = tokio::task::block_in_place(move || {
        tokio::runtime::Runtime::new().unwrap().block_on(
            get_symbols(asset_class, Category::All, Exchange::All)
       ).unwrap()   
       });
    let symbols = tickers
        .iter()
        .filter(|tc| tc.symbol.to_lowercase().contains(&query.to_lowercase())
            || tc.name.to_lowercase().contains(&query.to_lowercase()))
        .map(|tc| (tc.symbol.clone(), tc.name.clone()))
        .collect::<HashMap<String, String>>();
    Python::with_gil(|py| {
        let py_dict = PyDict::new(py);
        for (symbol, name) in symbols {
            py_dict.set_item(symbol, name).unwrap();
        }
        py_dict.into()
    })
}


#[pyfunction]
#[pyo3(name = "get_symbols")]
/// Fetches all ticker symbols
///
/// # Returns
///
/// `DataFrame` - a pandas dataframe of all ticker symbol details
///
/// # Example
///
/// ```
/// import yahoo_finance_symbols as ys
///
/// symbols_df = ys.get_symbols()
/// print(symbols_df)
/// ```
pub fn get_symbols_py() -> PyObject {
    let symbols = tokio::task::block_in_place(move || {
        tokio::runtime::Runtime::new().unwrap().block_on(
            get_symbols(AssetClass::All, Category::All, Exchange::All)
       ).unwrap()   
       });
    Python::with_gil(|py| {
        let pandas = py.import("pandas").expect("Failed to import pandas");
        let list_of_dicts = PyList::empty(py);

        for tc in symbols {
            let row = PyDict::new(py);
            row.set_item("symbol", tc.symbol).unwrap();
            row.set_item("name", tc.name).unwrap();
            row.set_item("asset_class", tc.asset_class).unwrap();
            row.set_item("category", tc.category).unwrap();
            row.set_item("exchange", tc.exchange).unwrap();
            list_of_dicts.append(row).unwrap();
        }

        let df = pandas
        .call_method1("DataFrame", (list_of_dicts,))
        .expect("Failed to create DataFrame");
    df.into()
    })
}

#[pyfunction]
#[pyo3(name = "update_database")]
pub fn update_database_py() {
    tokio::task::block_in_place(move || {
         tokio::runtime::Runtime::new().unwrap().block_on(
            update_database()
        ).unwrap()   
        })

}