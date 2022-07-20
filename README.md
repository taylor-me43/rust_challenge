# Rust Challenge

# Introduction

Given a CSV representing a series of transactions, implement a simple toy transactions engine that processes the payments crediting and debiting accounts. After processing the complete set of payments output the client account balances. You should be able to run your payments engine like:

```
$ cargo run -- transactions.csv > accounts.csv
```

The input file is the first and only argument to thebinary. Output should be written to std out.

# Input

The input will be a CSV file with the columns type, client, tx, and amount. You can assume the type is a string, the client column is a valid u16 client ID, the tx is a valid u32 transaction ID, and the amount is a decimal value with a precision of up to four places past the decimal.

For example

```
type,	client,	tx,	amount
deposit,	1,	1,	1.0
deposit,	2,	2,	2.0
deposit,	1,	3,	2.0
withdrawal,	1,	4,	1.5
withdrawal,	2,	5,	3.0
```

The client ID will be unique per client though are not guaranteed to be ordered. Transactions to the client account 2 could occur before transactions to the client account 1. Likewise, transaction IDs (tx) are globally unique, though are also not guaranteed to be ordered. You can assume the transactions occur chronologically in the file, so if transaction b appears after a in the input file then you can assume b occurred chronologically after a. Whitespaces and decimal precisions (up to four places past the decimal) must be accepted by your program.


# Output:


Output

The output should be a list of client IDs (client), available amounts (available), held amounts (held), total amounts (total), and whether the account is locked (locked). Columns are defined as

| Column  |  Description  
| ------------------- | ------------------- |
| available | The total funds that are available for trading, staking, withdrawal, etc. This should be equal to the total - held amounts |
| held | The total funds that are held for dispute. This should be equal to total - available amounts |
| total | The total funds that are available or held. This should be equal to available + held |
| locked | Whether the account is locked. An account is locked if a charge back occurs |

For example:

```
client, available, held, total, locked
1,	1.5, 0.0,	1.5, false
2,	2.0, 0.0,	2.0, false
```
Spacing and displaying decimals for round values do not matter. Row ordering also does not matter. The above output will be considered the exact same as the following:

```
client,available,held,total, 
2,2,0,2,false 
1,1.5,0,1.5,false
```
# Running:

Executing tests:

```
cargo test -- --show-output
```

Running:

```
cargo run -- input_test.csv
```

# Precision:

You can assume a precision of four places past the decimal and should output values with the same level of precision.

# Types of Transactions:

## Deposit:

A deposit is a credit to the client's asset account, meaning it should increase the available and total funds of the client account

A deposit looks like:

| type  |  client  |  tx  |  amount |
| ------------------- | ------------------- | ------------------- | ------------------- |
| deposit  |  1  |  1  |  1.0 |

## Withdrawal:

A withdraw is a debit to the client's asset account, meaning it should decrease the available and total funds of the client account

A withdrawal looks like:

| type  |  client  |  tx  |  amount |
| ------------------- | ------------------- | ------------------- | ------------------- |
| withdrawal  |  2  |  2  |  1.0 |

If a client does not have sufficient available funds the withdrawal should fail and the total amount of funds should not change.

## Dispute:

A dispute represents a client's claim that a transaction was erroneous and should be reversed. The transaction shouldn't be reversed yet but the associated funds should be held. This means that the clients available funds should decrease by the amount disputed, their held funds should increase by the amount disputed, while their total funds should remain the same.

A dispute looks like:

| type  |  client  |  tx  |  amount |
| ------------------- | ------------------- | ------------------- | ------------------- |
| dispute  |  1  |  1 | |

Notice that a dispute does not state the amount disputed. Instead a dispute references the transaction that is disputed by ID. If the tx specified by the dispute doesn't exist you can ignore it and assume this is an error on our partners side.

## Resolve:

A resolve represents a resolution to a dispute, releasing the associated held funds. Funds that were previously disputed are no longer disputed. This means that the clients held funds should decrease by the amount no longer disputed, their available funds should increase by the amount no longer disputed, and their total funds should remain the same.

A resolve looks like:

| type  |  client  |  tx  |  amount |
| ------------------- | ------------------- | ------------------- | ------------------- |
| resolve  |  1  |  1  |  |

Like disputes, resolves do not specify an amount. Instead they refer to a transaction that was under dispute by ID. If the tx specified doesn't exist, or the tx isn't under dispute, you can ignore the resolve and assume this is an error on our partner's side.


## Chargeback:

A chargeback is the final state of a dispute and represents the client reversing a transaction. Funds that were held have now been withdrawn. This means that the clients held funds and total funds should decrease by the amount previously disputed. If a chargeback occurs the client's account should be immediately frozen.

A chargeback looks like:

| type  |  client  |  tx  |  amount |
| ------------------- | ------------------- | ------------------- | ------------------- |
| chargeback  |  1  |  1 |

Like a dispute and a resolve a chargeback refers to the transaction by ID (tx) and does not specify an amount. Like a resolve, if the tx specified doesn't exist, or the tx isn't under dispute, you can ignore chargeback and assume this is an error on our partner's side.

# Assumptions:

- The client has a single asset account. All transactions are to and from this single asset account;
- There are multiple clients. Transactions reference clients. If a client doesn't exist create a new record;
- Clients are represented by u16 integers. No names, addresses, or complex client profile info;

# Extra Assumptions:

- The client has a single asset account. All transactions are to and from this single asset account;
- Invalid data on CSV entry (e.g. Operation, Client, Tx, Amount columns) should invalidate and skip transaction.
- Specific invalid operations should raise an error message, since these errors could be considered critical bugs or security incidents.
- Conflicting transactions output error messages and location, since incorrect account details could be generated by only skipping incorrect lines and continuing normal application flow.
- Disputes with unmatching Client ID and Transaction ID should raise an error message and location, since incorrect account details could be generated by only skipping incorrect lines and continuing normal application flow.
- Resolve/Chargeback operations without previous dispute should be skipped.
- Withdrawal/Dispute without registered client ID should be skipped.
- Withdrawal without fund should be skipped.

# Unit Tests:

The tests were divided in 3 sections: 

- Correct results: tests to verify correct processing.
- Critical errors: tests to verify error messages for critical issues, generally resulted of conflicting transactions or errors that could not be tracked to a single transaction.
- Normal errors: errors that can be skipped, since they are generated by only one incorrect transaction.

## Correct results:

- default_test(): Default Input
- four_decimal_places(): Amounts should be truncated with 4 decimal precision. 
- successful_dispute(): Unit test to verify correct processing of dispute transactions
- successful_resolve(): Unit test to verify correct processing of resolve transactions.
- successful_chargeback(): Unit test to verify correct processing of chargeback transactions.
- frozen_account_test(): Unit test to verify the account is frozen correctly and will not process any other transaction.

## Critical errors:

- invalid_operation(): Invalid Operation types should output an error.
- invalid_clientid(): Type mismatch on client column should raise an error message (e.g string value instead of integer).
- invalid_txid(): Type mismatch on transaction column should raise an error message (e.g string value instead of integer). 
- invalid_amount(): Type mismatch on amount column should raise an error message (e.g string value instead of integer). 
- conflicting_transaction(): Transactions with same ID should raise an error message.
- divergent_transaction_id(): Disputes with unmatching Client ID and Transaction ID should raise an error message.

## Normal errors:
- resolve_missing_dispute(): Resolve operations without previous dispute should be skipped.
- chargeback_missing_dispute(): Chargeback operations without previous dispute should be skipped.
- withdrawal_missing_clientid(): Withdrawal without Client ID record. The withdrawal with missing client ID record should not be processed.
- dispute_missing_clientid(): Dispute without previous client ID record should not be processed.
- withdrawal_without_funds(): Withdrawal without funds should be skipped.
