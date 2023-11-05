use clap::Parser;
use ledger_parser::{LedgerItem, Transaction};

use futures::stream::TryStreamExt;
use tokio::fs::File;
use tokio::io::BufReader;
use tokio_util::codec::{FramedRead, LinesCodec};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Transactions file path(s).
    #[clap(short, long, required = true, num_args = 1..)]
    files: Vec<String>,
}

async fn collect_transactions(
    transaction: &str,
) -> Result<Vec<Transaction>, Box<dyn std::error::Error>> {
    Ok(ledger_parser::parse(transaction)?
        .items
        .into_iter()
        .map(|item| match item {
            LedgerItem::Transaction(transaction) => {
                println!("{:?}", transaction);
                transaction
            },
            _ => todo!(),
        })
        .collect())
}

async fn parse_transactions(path: &str) -> Result<Vec<Transaction>, Box<dyn std::error::Error>> {
    let file = File::open(path).await?;
    let reader = BufReader::new(file);
    let mut transactions: Vec<Transaction> = Vec::new();
    let mut lines = FramedRead::new(reader, LinesCodec::new());
    let mut current_transaction = String::new();
    let mut separator_count = 0;

    while let Some(line) = lines.try_next().await? {
        if line.is_empty() {
            separator_count += 1;
            if separator_count == 2 {
                transactions.extend_from_slice(&collect_transactions(&current_transaction).await?);
                current_transaction.clear();
                separator_count = 0;
            }
        } else {
            // Append non-empty lines to the current transaction
            if !current_transaction.is_empty() {
                current_transaction.push('\n');
            }
            current_transaction.push_str(&line);
        }
    }

    if !current_transaction.is_empty() {
        transactions.extend_from_slice(&collect_transactions(&current_transaction).await?);
    }

    Ok(transactions)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    parse_transactions(&args.files[0]).await?; // on hyperfine and my machine, this takes 10ms
    Ok(())
}
