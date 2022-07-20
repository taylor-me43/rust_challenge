extern crate rust_csv;
use std::{fs::File, collections::HashMap};
use serde_derive::Deserialize;
mod transactions;
mod error;
use std::io::BufReader;
use rust_csv::{ReaderBuilder, Trim};
use crate::{transactions::operate_account};

/// Struct for processing CSV fields.
/// 
/// Invalid Fields are filtered and sanitized by the application.
#[derive(Deserialize)]
pub struct Input{
    #[serde(rename = "type",deserialize_with = "rust_csv::invalid_option")]
    op_type: Option<Operation>,
    #[serde(deserialize_with = "rust_csv::invalid_option")]
    client: Option<u16>,
    #[serde(deserialize_with = "rust_csv::invalid_option")]
    tx: Option<u32>,
    #[serde(deserialize_with = "rust_csv::invalid_option")]
    amount: Option<f32>
}

/// Struct used for keeping dispute information of Transactions
/// 
/// This struct is used to keep track of disputes in chargeback 
/// and resolve operations.

pub struct Txs{
    info: Input,
    in_dispute: bool
}

/// Available operations for the Wallet.

#[derive(Debug, Deserialize,PartialEq,Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Operation{
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback
}

#[derive(Debug, Deserialize)]
pub struct Funds(f64);

/// Trait used to ensure 4 decimal places in the stored amount
/// 
/// Avoids importing large crates (e.g. rust_decimal) for a simple operation.
/// 
/// WARNING: the application does not currently support rounding up/down, only performing truncation.
/// This is a known issue and depending on application requirements should be addressed.
trait FundAccount{
    fn get_amount(amount: f32) -> f32;
}

impl FundAccount for Funds {

    fn get_amount(amount: f32) -> f32 {
        (amount * 10000.0).round() / 10000.0
    }
}

/// Struct used for storing account information: 
/// 
/// Available, Held and Total amount. Also keeps  
/// track of the account state (locked/not locked). 

pub struct AccInfo{
    available: f32,
    held: f32,
    total: f32,
    locked: bool
}

/// if is_csv == true: process csv input
/// if is_csv == false: process string input as csv entry
pub fn csv_read(input: &str, is_csv: bool) -> Result<HashMap<u16,AccInfo>, String> {
    match is_csv{
        true => {
            let f = File::open(input).unwrap();
            let mut accs:HashMap<u16,AccInfo> = HashMap::new();
            let mut tx:HashMap<u32,Txs> = HashMap::new();
            let mut rd = ReaderBuilder::new()
                .trim(Trim::All)
                .flexible(true)
                .from_reader(BufReader::new(f));
            let mut it = 1;
            for result in rd.deserialize::<Input>() {
                let tr: Input = result.unwrap();
                match operate_account(tr, &mut accs, it, &mut tx){
                    Ok(()) => {it = it + 1},
                    Err(error) => return Err(error),
                }
            }
            Ok(accs)
        },
        false => {
            let mut accs:HashMap<u16,AccInfo> = HashMap::new();
            let mut tx:HashMap<u32,Txs> = HashMap::new();
            let mut rdr = ReaderBuilder::new().trim(Trim::All).from_reader(input.as_bytes());
            let mut line = 1;
            for result in rdr.deserialize::<Input>() {
                let tr: Input = result.unwrap();
                match operate_account(tr, &mut accs, line, &mut tx){
                    Ok(()) => {line = line +1},
                    Err(error) => return Err(error),
                }
            }
            Ok(accs)
        },
    }
}

/// Receive account details and format into csv table
pub fn fmt_output(accounts:HashMap<u16,AccInfo>)->String{
    let mut output = "client, available, held, total, locked".to_string();
    for (key, value) in accounts.into_iter() {
        output = format!("{}\n{}, {}, {}, {}, {}",output,key,value.available,value.held,value.total,value.locked.to_string());
    }
    output
}


/// Default Input.
/// 
/// Input:
/// 
/// `type, client, tx, amount`
/// 
/// `deposit, 1, 1, 1.0`
/// 
/// `deposit, 2, 2, 2.0`
/// 
/// `withdrawal, 1, 4, 1.5`
/// 
/// `withdrawal, 2, 5, 3.0`
/// 
/// Expected:
/// 
/// `client, available, held, total, locked`
/// 
/// `1, 1.5, 0.0, 1.5, false`
/// 
/// `2, 2.0, 0.0, 2.0, false`
#[test]
fn default_test() {
    let input = "
    type, client, tx, amount
    deposit, 1, 1, 1.0
    deposit, 2, 2, 2.0
    deposit, 1, 3, 2.0
    withdrawal, 1, 4, 1.5
    withdrawal, 2, 5, 3.0";

    let is_csv = false;

    let mut expected_hashmap: HashMap<u16,AccInfo> = HashMap::new();

    expected_hashmap.insert(1, AccInfo { available: 1.5, held: 0.0, total: 1.5, locked: false });
    expected_hashmap.insert(2, AccInfo { available: 2.0, held: 0.0, total: 2.0, locked: false });


    let output = match csv_read(&input, is_csv){
        Ok(accs) => std::result::Result::Ok(accs),
        Err(_e) =>  std::result::Result::Err(_e),
    };
    
    let result = output.ok().unwrap();
    for (key, value) in result {
        let expected_account_info = expected_hashmap.get(&key);
        match expected_account_info{
            Some(acc) => {
                assert_eq!(acc.total,value.total);
                assert_eq!(acc.held,value.held);
                assert_eq!(acc.locked,value.locked);
                assert_eq!(acc.available,value.available);
            }
            None => assert!(false),
        }
    }
}

/// Amounts should be truncated with 4 decimal precision. 
/// In a real scenario this should be analyzed wether to truncate
/// or round the amount, since it could impact on client or kraken losing
/// amounts in the transactions.
/// 
/// Input:
/// 
/// `type, client, tx, amount`
/// 
/// `deposit, 1, 1, 1.123456`
/// 
/// `deposit, 2, 2, 2.123456`
/// 
/// `deposit, 1, 3, 2.654321`
/// 
/// `withdrawal, 1, 4, 1.7654321`
/// 
/// `withdrawal, 2, 5, 1.5432345`
/// 
/// Expected:
/// 
/// `client, available, held, total, locked`
/// `1, 2.0124,0,2.0124,false`
/// 
/// `2, 0.8891, 0.0, 0.8891, false`
#[test]
fn four_decimal_places() {
    let input = "
    type, client, tx, amount
    deposit, 1, 1, 1.123456
    deposit, 2, 2, 2.123456
    deposit, 1, 3, 2.654321
    withdrawal, 1, 4, 1.7654321
    withdrawal, 2, 5, 1.5432345";

    let mut expected_hashmap: HashMap<u16,AccInfo> = HashMap::new();
    let is_csv = false;

    expected_hashmap.insert(1, AccInfo { available: 2.0124, held: 0.0, total: 2.0124, locked: false });
    expected_hashmap.insert(2, AccInfo { available: 0.5803, held: 0.0, total: 0.5803, locked: false });

    let output = match csv_read(&input, is_csv){
        Ok(accs) => std::result::Result::Ok(accs),
        Err(_e) =>  std::result::Result::Err(_e),
    };
    
    let result = output.ok().unwrap();
    for (key, value) in result {
        let expected_account_info = expected_hashmap.get(&key);
        match expected_account_info{
            Some(acc) => {
                assert_eq!(acc.total,value.total);
                assert_eq!(acc.held,value.held);
                assert_eq!(acc.locked,value.locked);
                assert_eq!(acc.available,value.available);
            }
            None => assert!(false),
        }
    }
}

/// Unit test to verify correct processing of dispute transactions
///
/// Input:
/// 
/// `type, client, tx, amount`
/// 
/// `deposit, 1, 1, 1.0`
/// 
/// `deposit, 2, 2, 2.0`
/// 
/// `deposit, 1, 3, 3.0`
/// 
/// `withdrawal, 1, 4, 2.5`
/// 
/// `withdrawal, 2, 5, 2.0`
/// 
/// `dispute, 1, 1, `
/// 
/// Expected:
/// 
/// `client, available, held, total, locked`
/// 
/// `1, 1.5, 1, 1.5, false`
/// 
/// `2, 2, 0, 2, false`
/// 
#[test]
fn successful_dispute() {
   
    let input = "
    type, client, tx, amount
    deposit, 1, 1, 1.0
    deposit, 2, 2, 2.0
    deposit, 1, 3, 3.0
    withdrawal, 1, 4, 2.5
    withdrawal, 2, 5, 2.0
    dispute, 1, 1, ";

    let mut expected_hashmap: HashMap<u16,AccInfo> = HashMap::new();
    let is_csv = false;


    expected_hashmap.insert(1, AccInfo { available: 0.5, held: 1.0, total: 1.5, locked: false });
    expected_hashmap.insert(2, AccInfo { available: 0.0, held: 0.0, total: 0.0, locked: false });

    let output = match csv_read(&input, is_csv){
        Ok(accs) => std::result::Result::Ok(accs),
        Err(_e) =>  std::result::Result::Err(_e),
    };
    
    let result = output.ok().unwrap();
    for (key, value) in result {
        let expected_account_info = expected_hashmap.get(&key);
        match expected_account_info{
            Some(acc) => {
                assert_eq!(acc.total,value.total);
                assert_eq!(acc.held,value.held);
                assert_eq!(acc.locked,value.locked);
                assert_eq!(acc.available,value.available);
            }
            None => assert!(false),
        }
    }
}

/// Unit test to verify correct processing of resolve transactions
/// 
/// Input:
/// 
/// `type, client, tx, amount`
/// 
/// `deposit, 1, 1, 1.0`
/// 
/// `deposit, 2, 2, 2.0`
/// 
/// `deposit, 1, 3, 3.0`
/// 
/// `withdrawal, 1, 4, 1.5`
/// 
/// `withdrawal, 2, 5, 3.0`
/// 
/// `dispute, 1, 1,`
/// 
/// `resolve, 1, 1, `
/// 
/// Expected:
/// 
/// `client, available, held, total, locked`
/// 
/// `1, 1.5, 0.0, 1.5, false`
/// 
/// `2, 0.0, 0.0, 0.0, false`
#[test]
fn successful_resolve() {
    let input = "
    type, client, tx, amount
    deposit, 1, 1, 1.0
    deposit, 2, 2, 2.0
    deposit, 1, 3, 3.0
    withdrawal, 1, 4, 2.5
    withdrawal, 2, 5, 2.0
    dispute, 1, 1, 
    resolve, 1, 1, ";

    let mut expected_hashmap: HashMap<u16,AccInfo> = HashMap::new();
    let is_csv = false;

    expected_hashmap.insert(1, AccInfo { available: 1.5, held: 0.0, total: 1.5, locked: false });
    expected_hashmap.insert(2, AccInfo { available: 0.0, held: 0.0, total: 0.0, locked: false });

    let output = match csv_read(&input, is_csv){
        Ok(accs) => std::result::Result::Ok(accs),
        Err(_e) =>  std::result::Result::Err(_e),
    };
    
    let result = output.ok().unwrap();
    for (key, value) in result {
        let expected_account_info = expected_hashmap.get(&key);
        match expected_account_info{
            Some(acc) => {
                assert_eq!(acc.total,value.total);
                assert_eq!(acc.held,value.held);
                assert_eq!(acc.locked,value.locked);
                assert_eq!(acc.available,value.available);
            }
            None => assert!(false),
        }
    }
}

/// Unit test to verify correct processing of chargeback transactions
/// 
/// Input:
/// 
/// `type, client, tx, amount`
/// 
/// `deposit, 1, 1, 1.0`
/// 
/// `deposit, 2, 2, 2.0`
/// 
/// `deposit, 1, 3, 3.0`
/// 
/// `withdrawal, 1, 4, 2.5`
/// 
/// `withdrawal, 2, 5, 2.0`
/// 
/// `dispute, 1, 1, `
/// 
/// `chargeback, 1, 1, `
/// 
/// Expected:
/// 
/// `client, available, held, total, locked`
/// 
/// `1, 0.5, 0.0, 0.5, true`
/// 
/// `2, 0.0, 0.0, 0.0, false`
#[test]
fn successful_chargeback() {

    let input = "
    type, client, tx, amount
    deposit, 1, 1, 1.0
    deposit, 2, 2, 2.0
    deposit, 1, 3, 3.0
    withdrawal, 1, 4, 2.5
    withdrawal, 2, 5, 2.0
    dispute, 1, 1, 
    chargeback, 1, 1, ";

    let mut expected_hashmap: HashMap<u16,AccInfo> = HashMap::new();
    let is_csv = false;

    expected_hashmap.insert(1, AccInfo { available: 0.5, held: 0.0, total: 0.5, locked: true });
    expected_hashmap.insert(2, AccInfo { available: 0.0, held: 0.0, total: 0.0, locked: false });

    let output = match csv_read(&input, is_csv){
        Ok(accs) => std::result::Result::Ok(accs),
        Err(_e) =>  std::result::Result::Err(_e),
    };
    
    let result = output.ok().unwrap();
    for (key, value) in result {
        let expected_account_info = expected_hashmap.get(&key);
        match expected_account_info{
            Some(acc) => {
                assert_eq!(acc.total,value.total);
                assert_eq!(acc.held,value.held);
                assert_eq!(acc.locked,value.locked);
                assert_eq!(acc.available,value.available);
            }
            None => assert!(false),
        }
    }
}

/// Unit test to verify the account is frozen correctly and will not process any other transaction.
/// 
/// Input:
/// 
/// `type, client, tx, amount`
/// 
/// `deposit, 1, 1, 1.0`
/// 
/// `deposit, 2, 2, 2.0`
/// 
/// `deposit, 1, 3, 3.0`
/// 
/// `withdrawal, 1, 4, 2.5`
/// 
/// `withdrawal, 2, 5, 2.0`
/// 
/// `dispute, 1, 1, `
/// 
/// `chargeback, 1, 1, `
/// 
/// `deposit, 1, 6, 2.0`
/// 
/// `withdrawal, 1, 7, 2.5`
/// 
/// `dispute, 1, 3, `
/// 
/// `resolve, 1, 3,`
/// 
/// `dispute, 1, 4, `
/// 
/// `chargeback, 1, 4, `
/// 
/// Expected:
/// 
/// `available, held, total, locked`
/// 
/// `0.5, 0.0, 0.5, true`
/// 
/// `0.0, 0.0, 0.0, false`
#[test]
fn frozen_account_test() {
    let input = "type, client, tx, amount
    deposit, 1, 1, 1.0
    deposit, 2, 2, 2.0
    deposit, 1, 3, 3.0
    withdrawal, 1, 4, 2.5
    withdrawal, 2, 5, 2.0
    dispute, 1, 1, 
    chargeback, 1, 1, 
    deposit, 1, 6, 2.0
    withdrawal, 1, 7, 2.5
    dispute, 1, 3, 
    resolve, 1, 3, 
    dispute, 1, 4, 
    chargeback, 1, 4, ";

    let mut expected_hashmap: HashMap<u16,AccInfo> = HashMap::new();
    let is_csv = false;

    expected_hashmap.insert(1, AccInfo { available: 0.5, held: 0.0, total: 0.5, locked: true });
    expected_hashmap.insert(2, AccInfo { available: 0.0, held: 0.0, total: 0.0, locked: false });

    let output = match csv_read(&input, is_csv){
        Ok(accs) => std::result::Result::Ok(accs),
        Err(_e) =>  std::result::Result::Err(_e),
    };
    
    let result = output.ok().unwrap();
    for (key, value) in result {
        let expected_account_info = expected_hashmap.get(&key);
        match expected_account_info{
            Some(acc) => {
                assert_eq!(acc.total,value.total);
                assert_eq!(acc.held,value.held);
                assert_eq!(acc.locked,value.locked);
                assert_eq!(acc.available,value.available);
            }
            None => assert!(false),
        }
    }
}

/// Invalid Operations should output an error. The application is 
/// pointing the location for the error.
/// This unexpected operation should be raised as an alert in a real scenario, since it could indicate
/// a bug within the application or a security incident.
#[test]
fn invalid_operation() {
    let input = "
    type, client, tx, amount
    deposit, 1, 1, 1.0
    test_operation, 1, 2, 3.0
    deposit, 1, 3, 3.0
    withdrawal, 1, 4, 2.5
    withdrawal, 2, 5, 3.0";

    let is_csv = false;

    let output = match csv_read(&input, is_csv){
        Ok(accs) => std::result::Result::Ok(accs),
        Err(_e) =>  std::result::Result::Err(_e),
    };

    assert_eq!(output.err().unwrap(),"Invalid Operation at line: 2");
}

/// Type mismatch on client column should raise an error message (e.g string value instead of integer). 
/// This unexpected operation should be raised as an alert in a real scenario, since it could indicate
/// a bug within the application or a security incident.
#[test]
fn invalid_clientid() {
    let input = "
    type, client, tx, amount
    deposit, 1, 1, 1.0
    deposit, 2, 2, 2.0
    deposit, invalid_client, 3, 3.0
    withdrawal, 1, 4, 1.5
    withdrawal, 2, 5, 2.0";

    let is_csv = false;

    let output = match csv_read(&input, is_csv){
        Ok(accs) => std::result::Result::Ok(accs),
        Err(_e) =>  std::result::Result::Err(_e),
    };

    assert_eq!(output.err().unwrap(),"Invalid Client at line: 3");
}

/// Type mismatch on transaction column should raise an error message (e.g string value instead of integer). 
/// This unexpected operation should be raised as an alert in a real scenario, since it could indicate
/// a bug within the application or a security incident.
#[test]
fn invalid_txid() {
    let expected_output = "
    type, client, tx, amount
    deposit, 1, 1, 1.0
    deposit, 2, 2, 2.0
    deposit, 1, 3, 2.0
    withdrawal, 1, invalid_tx, 1.5
    withdrawal, 2, 5, 3.0";

    let is_csv = false;

    let output = match csv_read(&expected_output, is_csv){
        Ok(accs) => std::result::Result::Ok(accs),
        Err(_e) =>  std::result::Result::Err(_e),
    };

    assert_eq!(output.err().unwrap(),"Invalid Tx at line: 4");

}

/// Type mismatch on amount column should raise an error message (e.g string value instead of integer). 
/// This unexpected operation should be raised as an alert in a real scenario, since it could indicate
/// a bug within the application or a security incident.
#[test]
fn invalid_amount() {
    let expected_output = "
    type, client, tx, amount
    deposit, 1, 1, 1.0
    deposit, 2, 2, 2.0
    deposit, 1, 3, 3.0
    withdrawal, 1, 4, 1.5
    withdrawal, 2, 5, invalid_amout";

    let is_csv = false;

    let output = match csv_read(&expected_output, is_csv){
        Ok(accs) => std::result::Result::Ok(accs),
        Err(_e) =>  std::result::Result::Err(_e),
    };

    assert_eq!(output.err().unwrap(),"Invalid Amount at line: 5");
}

/// Transactions with same ID should raise an error message, 
/// since this could indicate a security incident or a critical bug.
#[test]
fn conflicting_transaction() {
    let expected_output = "
    type, client, tx, amount
    deposit, 1, 1, 1.0
    deposit, 2, 2, 2.0
    deposit, 1, 2, 3.0
    withdrawal, 1, 4, 2.5
    withdrawal, 2, 5, 3.0";

    let is_csv = false;

    let output = match csv_read(&expected_output, is_csv){
        Ok(accs) => std::result::Result::Ok(accs),
        Err(_e) =>  std::result::Result::Err(_e),
    };

    assert_eq!(output.err().unwrap(),"Conflicting Transaction at line: 3");
}

/// Disputes with unmatching Client ID and Transaction ID should raise an error message, since this
/// could indicate a critical bug within the application or possible manipulation of data in a security incident.
#[test]
fn divergent_transaction_id() {
    let expected_output = "
    type, client, tx, amount
    deposit, 1, 1, 1.0
    deposit, 2, 2, 2.0
    deposit, 1, 3, 2.0
    withdrawal, 1, 4, 1.5
    withdrawal, 2, 5, 3.0
    dispute, 2, 1, ";

    let is_csv = false;

    let output = match csv_read(&expected_output, is_csv){
        Ok(accs) => std::result::Result::Ok(accs),
        Err(_e) =>  std::result::Result::Err(_e),
    };

    assert_eq!(output.err().unwrap(),"Divergent Transaction and Client ID at line: 6");
    
}

/// Resolve operations without previous dispute should be skipped
/// 
/// Input:
/// 
/// `type, client, tx, amount`
/// 
/// `deposit, 1, 1, 1.0`
/// 
/// `deposit, 2, 2, 2.0`
/// 
/// `deposit, 1, 3, 3.0`
/// 
/// `withdrawal, 1, 4, 2.5`
/// 
/// `withdrawal, 2, 5, 2.0`
/// 
/// `resolve, 1, 2, `
/// 
/// `deposit, 1, 6, 2.0 `
/// 
/// Expected:
/// 
/// `client, available, held, total, locked`
/// 
/// `1, 3.5, 0.0, 3.5, false`
/// 
/// `2, 0.0, 0.0, 0.0, false`
#[test]
fn resolve_missing_dispute() {
    let input = "
    type, client, tx, amount
    deposit, 1, 1, 1.0
    deposit, 2, 2, 2.0
    deposit, 1, 3, 3.0
    withdrawal, 1, 4, 2.5
    withdrawal, 2, 5, 2.0
    resolve, 2, 2, 
    deposit, 1, 6, 2.0";

    let mut expected_hashmap: HashMap<u16,AccInfo> = HashMap::new();
    let is_csv = false;

    expected_hashmap.insert(1, AccInfo { available: 3.5, held: 0.0, total: 3.5, locked: false });
    expected_hashmap.insert(2, AccInfo { available: 0.0, held: 0.0, total: 0.0, locked: false });

    let output = match csv_read(&input, is_csv){
        Ok(accs) => std::result::Result::Ok(accs),
        Err(_e) =>  std::result::Result::Err(_e),
    };
    
    let result = output.ok().unwrap();
    for (key, value) in result {
        let expected_account_info = expected_hashmap.get(&key);
        match expected_account_info{
            Some(acc) => {
                assert_eq!(acc.total,value.total);
                assert_eq!(acc.held,value.held);
                assert_eq!(acc.locked,value.locked);
                assert_eq!(acc.available,value.available);
            }
            None => assert!(false),
        }
    }
}

/// Chargeback operations without previous dispute should be skipped
/// 
/// Input:
/// 
/// `type, client, tx, amount`
/// 
/// `deposit, 1, 1, 1.0`
/// 
/// `deposit, 2, 2, 2.0`
/// 
/// `deposit, 1, 3, 3.0`
/// 
/// `withdrawal, 1, 4, 1.5`
/// 
/// `withdrawal, 2, 5, 2.0`
/// 
/// `chargeback, 1, 1, `
/// 
/// `deposit, 1, 6, 2.0 `
/// 
/// Expected:
/// 
/// `client, available, held, total, locked`
/// 
/// `1, 4.5, 0.0, 4.5, false`
/// 
/// `2, 0.0, 0.0, 0.0, false`
#[test]
fn chargeback_missing_dispute() {
    let input = "
    type, client, tx, amount
    deposit, 1, 1, 1.0
    deposit, 2, 2, 2.0
    deposit, 1, 3, 3.0
    withdrawal, 1, 4, 1.5
    withdrawal, 2, 5, 2.0
    chargeback, 1, 1, 
    deposit, 1, 6, 2.0";

    let mut expected_hashmap: HashMap<u16,AccInfo> = HashMap::new();
    let is_csv = false;

    expected_hashmap.insert(1, AccInfo { available: 4.5, held: 0.0, total: 4.5, locked: false });
    expected_hashmap.insert(2, AccInfo { available: 0.0, held: 0.0, total: 0.0, locked: false });

    let output = match csv_read(&input, is_csv){
        Ok(accs) => std::result::Result::Ok(accs),
        Err(_e) =>  std::result::Result::Err(_e),
    };
    
    let result = output.ok().unwrap();
    for (key, value) in result {
        let expected_account_info = expected_hashmap.get(&key);
        match expected_account_info{
            Some(acc) => {
                assert_eq!(acc.total,value.total);
                assert_eq!(acc.held,value.held);
                assert_eq!(acc.locked,value.locked);
                assert_eq!(acc.available,value.available);
            }
            None => assert!(false),
        }
    }
}

/// Withdrawal without Client ID record. The withdrawal with missing client ID record should not be processed.
/// Input: 
/// 
/// `type, client, tx, amount`
/// 
/// `deposit, 1, 1, 1.0`
/// 
/// `deposit, 1, 3, 3.0`
/// 
/// `withdrawal, 1, 4, 1.0`
/// 
/// `withdrawal, 2, 5, 2.0`
/// 
/// Expected:
/// 
/// `client, available, held, total, locked`
/// 
/// `1, 3.0, 0.0, 3.0, false`
#[test]
fn withdrawal_missing_clientid() {
    let input = "
    type, client, tx, amount
    deposit, 1, 1, 1.0
    deposit, 1, 3, 3.0
    withdrawal, 1, 4, 1.0
    withdrawal, 2, 5, 2.0";

    let mut expected_hashmap: HashMap<u16,AccInfo> = HashMap::new();
    let is_csv = false;
    
    expected_hashmap.insert(1, AccInfo { available: 3.0, held: 0.0, total: 3.0, locked: false });

    let output = match csv_read(&input, is_csv){
        Ok(accs) => std::result::Result::Ok(accs),
        Err(_e) =>  std::result::Result::Err(_e),
    };
    
    let result = output.ok().unwrap();
    for (key, value) in result {
        let expected_account_info = expected_hashmap.get(&key);
        match expected_account_info{
            Some(acc) => {
                assert_eq!(acc.total,value.total);
                assert_eq!(acc.held,value.held);
                assert_eq!(acc.locked,value.locked);
                assert_eq!(acc.available,value.available);
            }
            None => assert!(false),
        }
    }
}

/// Dispute without previous client ID record should not be processed.
/// 
/// Input:
/// 
/// `type, client, tx, amount`
/// 
/// `deposit, 1, 1, 1.0`
/// 
/// `deposit, 1, 3, 2.5`
/// 
/// `withdrawal, 1, 4, 1.5`
/// 
/// `dispute, 2, 5, `
/// 
/// Expected:
/// 
/// `client, available, held, total, locked`
/// 
/// `1, 2.0, 0.0, 2.0, false`
#[test]
fn dispute_missing_clientid() {
    let input = "
    type, client, tx, amount
    deposit, 1, 1, 1.0
    deposit, 1, 3, 2.5
    withdrawal, 1, 4, 1.5
    dispute, 2, 5, ";

    let mut expected_hashmap: HashMap<u16,AccInfo> = HashMap::new();
    let is_csv = false;

    expected_hashmap.insert(1, AccInfo { available: 2.0, held: 0.0, total: 2.0, locked: false });
    
    let output = match csv_read(&input, is_csv){
        Ok(accs) => std::result::Result::Ok(accs),
        Err(_e) =>  std::result::Result::Err(_e),
    };
    
    let result = output.ok().unwrap();
    for (key, value) in result {
        let expected_account_info = expected_hashmap.get(&key);
        match expected_account_info{
            Some(acc) => {
                assert_eq!(acc.total,value.total);
                assert_eq!(acc.held,value.held);
                assert_eq!(acc.locked,value.locked);
                assert_eq!(acc.available,value.available);
            }
            None => assert!(false),
        }
    }
}

/// Withdrawal without fund should be skipped.
/// 
/// Input:
/// 
/// `type, client, tx, amount`
/// 
/// `deposit, 1, 1, 1.0`
/// 
/// `deposit, 1, 3, 2.5`
/// 
/// `withdrawal, 1, 4, 4.5`
/// 
/// Expected:
/// 
/// `client, available, held, total, locked`
/// 
/// `1, 3.5, 0.0, 3.5, false`
#[test]
fn withdrawal_without_funds() {
    let input = "
    type, client, tx, amount
    deposit, 1, 1, 1.0
    deposit, 1, 3, 2.5
    withdrawal, 1, 4, 4.5";

    let mut expected_hashmap: HashMap<u16,AccInfo> = HashMap::new();
    let is_csv = false;

    expected_hashmap.insert(1, AccInfo { available: 3.5, held: 0.0, total: 3.5, locked: false });
    
    let output = match csv_read(&input, is_csv){
        Ok(accs) => std::result::Result::Ok(accs),
        Err(_e) =>  std::result::Result::Err(_e),
    };
    
    let result = output.ok().unwrap();
    for (key, value) in result {
        let expected_account_info = expected_hashmap.get(&key);
        match expected_account_info{
            Some(acc) => {
                assert_eq!(acc.total,value.total);
                assert_eq!(acc.held,value.held);
                assert_eq!(acc.locked,value.locked);
                assert_eq!(acc.available,value.available);
            }
            None => assert!(false),
        }
    }
}
