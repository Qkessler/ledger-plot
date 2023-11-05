use std::collections::HashMap;

use ledger_parser::{Posting, Transaction};
use rust_decimal::Decimal;

// for each account, I need to know the postings individually
// with their dates, but also need to know the currency, because it's possible
// for the same account, we have different currencies (though rare).
#[derive(Default, Debug)]
pub struct Accounts {
    pub postings_per_account: HashMap<String, Vec<Decimal>>,
}

impl Accounts {
    pub fn new(postings_per_account: HashMap<String, Vec<Decimal>>) -> Accounts {
        Accounts {
            postings_per_account,
        }
    }

    pub fn update_accounts(&mut self, transaction: Transaction) {
        let mut zero_sum_count = Decimal::ZERO;
        let mut zero_sum_posting: Option<Posting> = None;
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
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use ledger_parser::{Amount, Commodity, CommodityPosition, PostingAmount, Reality};
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
