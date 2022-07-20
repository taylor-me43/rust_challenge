use std::env;
use rust_coding_test::csv_read;
use rust_coding_test::fmt_output;
fn main() {
    let arguments: Vec<String> = env::args().collect();
    let mut is_csv = true;
    let input: &str= match arguments.len() {
        1 => {
            //This is used in case arguments are not provided
            //Allows manipulating input directly through code, 
            //without needing a local CSV file.
            is_csv = false;
            let input_string = "
            type, client, tx, amount
            deposit, 1, 1, 1.0
            deposit, 2, 2, 2.0
            deposit, 1, 3, 2.0
            withdrawal, 1, 4, 1.5
            withdrawal, 2, 5, 3.0";
            &input_string
        }
        _ => {
            let input_csv = &arguments[1];
            &input_csv
        }
    };


    match csv_read(&input, is_csv){
        Ok(accs) => println!("{}", fmt_output(accs)),
        Err(_e) => println!("{}", _e),
    }
}
