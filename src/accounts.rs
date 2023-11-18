use anyhow::Context;
use ledger_parser::Transaction;
use plotters::prelude::*;
use plotters_backend::DrawingBackend;
use rust_decimal::prelude::*;
use std::collections::HashMap;

// for each account, I need to know the postings individually
// with their dates, but also need to know the currency, because it's possible
// for the same account, we have different currencies (though rare).
#[derive(Default, Debug)]
pub struct Accounts {
    pub postings_per_account: HashMap<String, Vec<Decimal>>,
}

impl Accounts {
    #[cfg(test)]
    pub fn new(postings_per_account: HashMap<String, Vec<Decimal>>) -> Accounts {
        Accounts {
            postings_per_account,
        }
    }

    pub fn update_accounts(&mut self, transaction: Transaction) {
        let mut zero_sum_count = Decimal::ZERO;
        let mut zero_sum_posting = None;
        for posting in transaction.postings {
            if posting.amount == None {
                zero_sum_posting = Some(posting);
                continue;
            }
            let quantity = posting.amount.expect("it's some").amount.quantity;
            self.postings_per_account
                .entry(posting.account)
                .or_default()
                .push(quantity);
            zero_sum_count -= quantity;
        }

        if let Some(posting) = zero_sum_posting {
            self.postings_per_account
                .entry(posting.account)
                .or_default()
                .push(zero_sum_count);
        }
    }

    pub fn draw_balance_for_account<DB: DrawingBackend>(
        &self,
        b: DrawingArea<DB, plotters::coord::Shift>,
        account: &str,
    ) -> anyhow::Result<()>
    where
        DB::ErrorType: 'static,
    {
        // TODO: we don't have the starting balance for the accounts. Can we run Ledger for that?
        // probably we can assume that the users have an opening balance for their accounts, otherwise they
        // won't have any meaningful reports.
        let postings = self
            .postings_per_account
            .get(account)
            .with_context(|| format!("account {} could not be found", account))?;
        println!("{:?}", &postings);

        // REVIEW: Something to probably optimize on insert if we wanted to.
        // REVIEW: What happens if we have negative max and min postings? We want to consider cases like Income,
        // where we might fill say, a Checking account using an Income account, which ends up negative with zero sum.
        let max_posting = postings
            .iter()
            .max()
            .expect("if account is present, there should be a posting");
        let min_posting = postings
            .iter()
            .min()
            .expect("if account is present, there should be a posting");
        println!("{:?}, {:?}", max_posting, min_posting);

        let y_max = max_posting.to_i32().expect("Decimal posting should work") + 10;
        let y_min = min_posting.to_i32().expect("Decimal posting should work") - 10;
        println!("y_max {y_max}, y_min {y_min}");

        let mut chart = ChartBuilder::on(&b)
            .x_label_area_size(35)
            .y_label_area_size(40)
            .margin(10)
            .caption(
                format!("Balance for {}", account),
                ("sans-serif", (6).percent_height()),
            )
            .build_cartesian_2d((0u32..10u32).into_segmented(), y_min..y_max)?;

        chart
            .configure_mesh()
            // .disable_mesh()
            .bold_line_style(WHITE.mix(0.3))
            .y_desc("Balance")
            .x_desc("Time")
            .axis_desc_style(("sans-serif", (4).percent_height()))
            .draw()?;

        // draw the line series or time series

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use ledger_parser::{Amount, Commodity, CommodityPosition, Posting, PostingAmount, Reality};
    use map_macro::hash_map;

    use super::*;

    const ASSETS_CHECKING: &str = "Assets:Checking";
    const GROCERIES_ACCOUNT: &str = "Expenses:Food:Groceries";

    fn create_posting_with_account_and_amount(account: &str, amount: Option<Decimal>) -> Posting {
        return match amount {
            None => Posting {
                account: account.to_owned(),
                reality: Reality::Real,
                amount: None,
                balance: None,
                status: None,
                comment: None,
            },
            Some(amount) => Posting {
                account: account.to_owned(),
                reality: Reality::Real,
                amount: Some(PostingAmount {
                    amount: Amount {
                        quantity: amount,
                        commodity: Commodity {
                            name: "€".to_owned(),
                            position: CommodityPosition::Right,
                        },
                    },
                    lot_price: None,
                    price: None,
                }),
                balance: None,
                status: None,
                comment: None,
            },
        };
    }

    fn create_transaction_with_postings(postings: Vec<Posting>) -> Transaction {
        Transaction {
            comment: None,
            date: Utc::now().date_naive(),
            effective_date: None,
            status: None,
            code: None,
            description: "test transaction".to_owned(),
            postings,
        }
    }

    #[test]
    fn when_update_accounts_has_multiple_postings_zero_based_sum_is_considered() {
        let mut accounts = Accounts::new(hash_map! {
            ASSETS_CHECKING.to_owned() => Vec::new(),
            GROCERIES_ACCOUNT.to_owned() => Vec::new(),
        });
        let transaction = create_transaction_with_postings(vec![
            create_posting_with_account_and_amount(ASSETS_CHECKING, Some(Decimal::NEGATIVE_ONE)),
            create_posting_with_account_and_amount(GROCERIES_ACCOUNT, None),
        ]);

        accounts.update_accounts(transaction);

        assert_eq!(
            accounts.postings_per_account.get(GROCERIES_ACCOUNT),
            Some(&vec![Decimal::ONE])
        );
    }

    #[test]
    fn when_transaction_has_same_account_twice_update_accounts_updates_it_twice() {
        let mut accounts = Accounts::new(hash_map! {
            ASSETS_CHECKING.to_owned() => Vec::new(),
            GROCERIES_ACCOUNT.to_owned() => Vec::new(),
        });
        let transaction = create_transaction_with_postings(vec![
            create_posting_with_account_and_amount(ASSETS_CHECKING, Some(Decimal::NEGATIVE_ONE)),
            create_posting_with_account_and_amount(ASSETS_CHECKING, Some(Decimal::NEGATIVE_ONE)),
            create_posting_with_account_and_amount(GROCERIES_ACCOUNT, None),
        ]);

        accounts.update_accounts(transaction);

        assert_eq!(
            accounts.postings_per_account.get(ASSETS_CHECKING),
            Some(&vec![Decimal::NEGATIVE_ONE, Decimal::NEGATIVE_ONE])
        );
    }

    #[test]
    fn when_all_transactions_have_amounts_update_accounts_updates_all() {
        let mut accounts = Accounts::new(hash_map! {
            ASSETS_CHECKING.to_owned() => Vec::new(),
            GROCERIES_ACCOUNT.to_owned() => Vec::new(),
        });
        let transaction = create_transaction_with_postings(vec![
            create_posting_with_account_and_amount(ASSETS_CHECKING, Some(Decimal::NEGATIVE_ONE)),
            create_posting_with_account_and_amount(GROCERIES_ACCOUNT, Some(Decimal::ONE)),
        ]);

        accounts.update_accounts(transaction);

        assert_eq!(
            accounts.postings_per_account.get(ASSETS_CHECKING),
            Some(&vec![Decimal::NEGATIVE_ONE])
        );
        assert_eq!(
            accounts.postings_per_account.get(GROCERIES_ACCOUNT),
            Some(&vec![Decimal::ONE])
        );
    }
}
