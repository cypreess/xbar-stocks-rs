use stock_checker_rs::fetch_latest_price;
use rayon::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
struct Position {
    ticker: String,
    buy_price: f64,
    shares: f64,
}

fn load_positions_from_csv(file_path: &str) -> Result<Vec<Position>, Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let mut reader = csv::Reader::from_reader(file);
    let mut positions = Vec::new();

    for result in reader.deserialize() {
        let position: Position = result?;
        positions.push(position);
    }

    Ok(positions)
}

fn get_csv_path() -> PathBuf {
    // Check command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        return PathBuf::from(&args[1]);
    }

    // Default to ~/.stocks/data.csv
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".stocks").join("data.csv")
}

fn format_with_separator(value: f64) -> String {
    let abs_value = value.abs();
    let integer_part = abs_value as i64;

    // Convert to string and add space separators
    let int_str = integer_part.to_string();
    let mut result = String::new();
    let len = int_str.len();

    for (i, ch) in int_str.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(' ');
        }
        result.push(ch);
    }

    result
}

fn consolidate_positions(positions: Vec<Position>) -> Vec<Position> {
    let mut consolidated: HashMap<String, (f64, f64)> = HashMap::new();

    // Accumulate total cost and total shares per ticker
    for position in positions {
        let entry = consolidated.entry(position.ticker).or_insert((0.0, 0.0));
        entry.0 += position.buy_price * position.shares; // total cost
        entry.1 += position.shares; // total shares
    }

    // Calculate weighted average buy price for each ticker
    consolidated
        .into_iter()
        .map(|(ticker, (total_cost, total_shares))| {
            Position {
                ticker,
                buy_price: total_cost / total_shares,
                shares: total_shares,
            }
        })
        .collect()
}

fn main() {
    // Get CSV file path from command line or use default
    let csv_path = get_csv_path();
    let csv_path_str = csv_path.to_str().unwrap_or("data.csv");

    // Load positions from CSV
    let positions = match load_positions_from_csv(csv_path_str) {
        Ok(positions) => positions,
        Err(e) => {
            eprintln!("Error loading positions from {}: {}", csv_path_str, e);
            eprintln!("Usage: {} [path/to/data.csv]", env::args().next().unwrap_or_else(|| "stock-checker-rs".to_string()));
            eprintln!("Default location: ~/.stocks/data.csv");
            std::process::exit(1);
        }
    };

    // Consolidate positions with same ticker (weighted average buy price)
    let consolidated_positions = consolidate_positions(positions);

    // Create a custom thread pool with limited parallelism to avoid overwhelming the server
    // Limit to 3 concurrent connections
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(7)
        .build()
        .unwrap();

    // Fetch all stocks in parallel using rayon with limited concurrency
    let results: Vec<_> = pool.install(|| {
        consolidated_positions
            .par_iter()
            .map(|position| {
                // Strip .US suffix for Yahoo Finance API
                let result = fetch_latest_price(&position.ticker);
                (position.clone(), result)
            })
            .collect()
    });

    // Calculate totals and prepare output with sorting
    let mut total_investment = 0.0;
    let mut total_current_value = 0.0;
    let mut position_data = Vec::new();

    for (position, result) in &results {
        let investment = position.buy_price * position.shares;
        total_investment += investment;

        match result {
            Ok(current_price) => {
                let current_value = current_price * position.shares;
                let change_percent = ((current_price - position.buy_price) / position.buy_price) * 100.0;
                let profit_loss = current_value - investment;

                total_current_value += current_value;

                position_data.push((
                    position.ticker.clone(),
                    position.buy_price,
                    *current_price,
                    change_percent,
                    profit_loss,
                    None, // No error
                ));
            }
            Err(e) => {
                position_data.push((
                    position.ticker.clone(),
                    position.buy_price,
                    0.0, // placeholder
                    f64::NEG_INFINITY, // sort errors to bottom
                    0.0, // placeholder
                    Some(e.to_string()),
                ));
            }
        }
    }

    // Sort by percentage change (highest to lowest)
    position_data.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

    // Generate output lines from sorted data
    let mut position_lines = Vec::new();
    for (ticker, buy_price, current_price, change_percent, profit_loss, error) in position_data {
        if let Some(err_msg) = error {
            position_lines.push(format!("{}: Error - {} | color=darkred", ticker, err_msg));
        } else {
            let sign = if profit_loss >= 0.0 { "+" } else { "-" };
            let color = if profit_loss >= 0.0 { "green" } else { "darkred" };

            // Format with padding for alignment
            let profit_str = format!("{}${}", sign, format_with_separator(profit_loss));
            let percent_str = format!("({}{:.2}%)",
                if change_percent >= 0.0 { "+" } else { "" },
                change_percent);

            position_lines.push(format!(
                "{:<10} ${:.2} @ ${:.2} {:>11} {:>10} | color={}",
                ticker,
                buy_price,
                current_price,
                profit_str,
                percent_str,
                color
            ));
        }
    }

    // Display in xbar format
    let total_profit_loss = total_current_value - total_investment;
    let total_change_percent = ((total_current_value - total_investment) / total_investment) * 100.0;

    // First line: appears in menu bar
    println!(
        "{}${} ({}{:.2}%)",
        if total_profit_loss >= 0.0 { "+" } else { "-" },
        format_with_separator(total_profit_loss),
        if total_change_percent >= 0.0 { "+" } else { "" },
        total_change_percent
    );

    // Separator for dropdown menu
    println!("---");
    //
    // // Portfolio summary
    println!("Investment: ${} | color=white", format_with_separator(total_investment));
    println!("Current: ${} | color=white", format_with_separator(total_current_value));
    println!("---");
    //
    // Individual positions
    for line in position_lines {
        println!("{}", line);
    }
}
