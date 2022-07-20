use std::{collections::{HashMap, hash_map::Entry}};
use crate::{Operation, error::Errors,AccInfo, Funds, FundAccount, Txs, Input};

pub fn deposit(row: Input, accounts: &mut HashMap<u16,AccInfo>,transactions: &mut HashMap<u32,Txs>,line:i32)->Result<(), String> {
    let amount = Funds::get_amount(row.amount.unwrap());
    let client = row.client;
    match transactions.entry(row.tx.unwrap()) {
        Entry::Occupied(_e) => {
            return Err(Errors::ConflictTransaction(line.to_string()).to_string());
        },
        Entry::Vacant(e) => {
            //New Transaction added
            e.insert(Txs { info: row, in_dispute: false });
        }
    }
    match accounts.entry(client.unwrap()) {
        Entry::Vacant(e) => {
            //No account record, creating new Account
            e.insert(AccInfo { available: amount, held: 0.0, total: amount, locked: false });
        },
        Entry::Occupied(mut e) => {
            //Found Account record: update
            if !e.get_mut().locked{
                e.get_mut().available = Funds::get_amount(e.get().available + amount);
                e.get_mut().total = Funds::get_amount(e.get().total + amount);
            }
        }
    }
    Ok(())
}

pub fn withdrawal(row: Input, accounts: &mut HashMap<u16,AccInfo>,transactions: &mut HashMap<u32,Txs>,line:i32)->Result<(), String> {
    let amount = Funds::get_amount(row.amount.unwrap());
    let client = row.client;
        match transactions.entry(row.tx.unwrap()) {
        Entry::Occupied(mut _e) => {
            return Err(Errors::ConflictTransaction(line.to_string()).to_string());
        },
        Entry::Vacant(e) => {
            //Create new TX
            e.insert(Txs { info: row, in_dispute: false });
        }
    }
    match accounts.entry(client.unwrap()) {
        Entry::Vacant(_e) => {
            //Account not found: withdrawal is not processed
            return Ok(())
        },
        Entry::Occupied(mut e) => {
            //Update account record
            if !e.get_mut().locked{
                //Verify if account has funds/is locked
                if e.get_mut().available >= amount{
                    e.get_mut().available = Funds::get_amount(e.get().available - amount);
                    e.get_mut().total = Funds::get_amount(e.get().total - amount);
                }
            }
        }
    }
    Ok(())
}

pub fn dispute(row: Input, accounts: &mut HashMap<u16,AccInfo>,transactions: &mut HashMap<u32,Txs>,line:i32)->Result<(), String> {
    match transactions.entry(row.tx.unwrap()) {
        Entry::Occupied(mut e) => {
            if e.get().info.client == row.client{
                //Check if clientId and tx in row match clientId and tx at HashMap 
                if !accounts.get(&row.client.unwrap()).unwrap().locked{
                    //Update account: under dispute
                    e.get_mut().in_dispute = true;
                    let new_account_value = accounts.get_mut(&row.client.unwrap()).unwrap();
                    new_account_value.available = Funds::get_amount(new_account_value.available - e.get().info.amount.unwrap());
                    new_account_value.held = Funds::get_amount(new_account_value.held + e.get().info.amount.unwrap());
                }
            }else{
                return Err(Errors::SecurityErrDivergentClientId(line.to_string()).to_string())
            }
        },
        Entry::Vacant(_) => {
            //TX not found
        },
    }
    Ok(())
}

pub fn resolve(row: Input, accounts: &mut HashMap<u16,AccInfo>,transactions: &mut HashMap<u32,Txs>,line:i32)->Result<(), String> {
    match transactions.entry(row.tx.unwrap()) {
        Entry::Occupied(mut e) => {
            match e.get().in_dispute{
                true => {
                    if e.get().info.client == row.client {
                        if !accounts.get(&row.client.unwrap()).unwrap().locked{
                            //Check if clientId and tx in row match clientId and tx at HashMap 
                            e.get_mut().in_dispute = false;
                            //Update: not under dispute anymore
                            let new_account_value = accounts.get_mut(&row.client.unwrap()).unwrap();
                            new_account_value.available = Funds::get_amount(new_account_value.available + e.get().info.amount.unwrap());
                            new_account_value.held = Funds::get_amount(new_account_value.held - e.get().info.amount.unwrap());
                        }
                    }else{
                        return Err(Errors::SecurityErrDivergentClientId(line.to_string()).to_string())
                    }
                }
                false => {
                    //TX not previously under dispute, skip
                },
            }
        },
        Entry::Vacant(_) => {
            //TX not found
        },
    }
    Ok(())
}

pub fn chargeback(row: Input, accounts: &mut HashMap<u16,AccInfo>,transactions: &mut HashMap<u32,Txs>,line:i32)->Result<(), String> {
    match transactions.entry(row.tx.unwrap()) {
        Entry::Occupied(mut e) => {
            match e.get().in_dispute{
                true => {
                    if e.get().info.client == row.client {
                        if !accounts.get(&row.client.unwrap()).unwrap().locked{
                        //Check if clientId and tx in row match clientId and tx at HashMap 
                            e.get_mut().in_dispute = false;
                            let new_account_value = accounts.get_mut(&row.client.unwrap()).unwrap();
                            new_account_value.held = Funds::get_amount(new_account_value.held - e.get().info.amount.unwrap());
                            new_account_value.total = Funds::get_amount(new_account_value.total - e.get().info.amount.unwrap());
                            new_account_value.locked = true;
                        }
                    }else{
                        return Err(Errors::SecurityErrDivergentClientId(line.to_string()).to_string())
                    }
                }
                false => {
                    //TX not previously under dispute, skip
                },
            }
        },
        Entry::Vacant(_) => {
            //TX not found
        },
    }
    Ok(())
}

pub fn operate_account(row: Input, accs: &mut HashMap<u16,AccInfo>, line: i32,txs: &mut HashMap<u32,Txs>)  -> Result<(), String> {
    let op_type = match row.op_type{
        Some(op) => op,
        None => return Err(Errors::InvalidOperation(line.to_string()).to_string()),
    };
    match row.client{
        Some(_) => {},
        None => return Err(Errors::InvalidClient(line.to_string()).to_string()),
    };
    match row.tx{
        Some(_) => {},
        None => return Err(Errors::InvalidTx(line.to_string()).to_string()),
    };
    match op_type {
        Operation::Deposit=> {
            match row.amount{
                Some(amount) => amount,
                None => return Err(Errors::InvalidAmount(line.to_string()).to_string()),
            };
            return deposit(row, accs,txs,line);
        },
        Operation::Withdrawal=> {
            match row.amount{
                Some(amount) => amount,
                None => return Err(Errors::InvalidAmount(line.to_string()).to_string()),
            };
            return withdrawal(row,accs,txs,line)
        },
        Operation::Dispute=> {
            dispute(row,accs,txs,line)
        },
        Operation::Resolve => {
            resolve(row,accs,txs,line)
        },
        Operation::Chargeback => {
            chargeback(row,accs,txs,line)
        }
    }
    
}
