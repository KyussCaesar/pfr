#![allow(non_camel_case_types)]

use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::env;
use std::str::FromStr;
use std::fmt;

#[macro_use] extern crate structopt;
#[macro_use] extern crate clap;
#[macro_use] extern crate serde_derive;

extern crate serde;
extern crate serde_json;

use structopt::StructOpt;

#[derive(StructOpt)]
/// personal finance reporter.
enum Commands
{
    /// init the list of entries.
    init,

    /// add a new entry.
    add(Transaction),

    /// remove an existing entry.
    rm(RmCommand),

    /// list the current entries.
    list,

    /// generate a report for the month
    report,
}


#[derive(StructOpt, Serialize, Deserialize)]
struct Transaction
{
    #[structopt(raw(possible_values = "&AddType::variants()", case_insensitive = "true"))]
    /// is this transaction an income or an expense?
    add_type: AddType,

    #[structopt(raw(possible_values = "&Frequency::variants()", case_insensitive = "true"))]
    /// how often does this transaction happen?
    freq: Frequency,

    /// the name of the transaction.
    name: String,

    /// the amount for this transaction.
    amount: Money,

    #[structopt(long = "category")]
    /// (for expenses) set the category for this transaction
    category: Option<String>,

    #[structopt(long = "account")]
    /// (for expenses) set the account that this expense comes from
    account: Option<String>,
}

#[derive(StructOpt)]
struct RmCommand
{
    /// the name of the entry to remove
    name: String
}

arg_enum!
{
    #[derive(Debug, Serialize, Deserialize)]
    /// Represents how often a transaction occurs.
    enum Frequency
    {
        daily,
        wkly,
        mthly,
        qtrly,
        yrly
    }
}


arg_enum!
{
    #[derive(Debug, Serialize, Deserialize)]
    /// Represents the type of transaction
    enum AddType
    {
        income,
        expense
    }
}


#[derive(StructOpt, Serialize, Deserialize, Debug)]
struct Money
{
    cents: u64
}


impl FromStr for Money
{
    type Err = std::num::ParseFloatError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err>
    {
        let float = f64::from_str(s)?;
        return Ok(Money { cents: (float * 100.0) as u64 });
    }
}


impl fmt::Display for Money
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        let dollars: u64 = self.cents / 100;
        let cents: u64 = self.cents % 100;
        return write!(f, "{:>4}.{:0>2}", dollars.to_string(), cents.to_string());
    }
}


fn main()
{
    let errors = match Commands::from_args()
    {
        Commands::init             => init(),
        Commands::add(transaction) => add(transaction),
        Commands::rm(transaction)  => rm(transaction),
        Commands::list             => list(),
        Commands::report           => report(),
    };

    // report error if there was one.
    errors.err().and_then(report_error);
}


/// Result alias.
type Result<T> = std::result::Result<T, Error>;


/// Error enum
/// Encapsulates all the ways things can go wrong.
enum Error
{
    WhileAttemptingToOpenDataFile(std::io::Error),
    DuringSerialisation(serde_json::Error),
    DuringDeSerialisation(serde_json::Error),
    CouldNotFindHomeDirectory,
    NameIsAlreadyTaken(String),
}


/// Prints a description of an error that has occurred.
fn report_error(e: Error) -> Option<()>
{
    use self::Error::*;

    print!("An error occurred");

    match e
    {
        WhileAttemptingToOpenDataFile(io_e) => println!(" while attempting to open the data file: {}", io_e),
        DuringSerialisation(e)              => println!(" while attempting to save to the data file: {}", e),
        DuringDeSerialisation(e)            => println!(" while attempting to load from the data file: {}", e),
        CouldNotFindHomeDirectory           => println!(" while attempting to find the current user's home directory; couldn't find it"),
        NameIsAlreadyTaken(s)               => println!(": a transaction called {} is already present in the ledger", s)
    }

    return None;
}


/// `Ledger`, just an alias for a hashmap.
type Ledger = HashMap<String, Transaction>;


/// Tries to open the pfr data file, located in the user's home directory.
///
/// If successful, returns `Ok(file_handle)`, otherwise, returns a variant of the
/// Error type describing the error that occurred.
fn get_file_handle(truncate: bool) -> Result<File>
{
    let mut home_dir = env::home_dir().ok_or_else(|| Error::CouldNotFindHomeDirectory)?;

    home_dir.push(".pfr_data");

    return OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(truncate)
        .open(home_dir)
        .map_err(|e| Error::WhileAttemptingToOpenDataFile(e));
}


/// Loads the ledger from the pfr data file.
fn load() -> Result<Ledger>
{
    serde_json::from_reader(get_file_handle(false)?).map_err(|e| Error::DuringDeSerialisation(e))
}


/// Saves the ledger to the pfr data file.
fn save(ledger: Ledger) -> Result<()>
{
    serde_json::to_writer_pretty(get_file_handle(true)?, &ledger).map_err(|e| Error::DuringSerialisation(e))
}


/// Creates the pfr data file in the user's home directory.
fn init() -> Result<()>
{
    save(Ledger::new())
}


/// Adds a new entry to the ledger.
/// Errors if an entry with the given name already exists.
fn add(ac: Transaction) -> Result<()>
{
    let mut ledger = load()?;

    return match ledger.insert(ac.name.clone(), ac)
    {
        Some(val) =>
        {
            let e = Error::NameIsAlreadyTaken(val.name.clone());
            ledger.insert(val.name.clone(), val);
            Err(e)
        },
        
        None => save(ledger),
    }
}


/// Removes an entry from the ledger.
fn rm(rc: RmCommand) -> Result<()>
{
    let mut ledger = load()?;
    ledger.remove(&rc.name);
    save(ledger)?;

    Ok(())
}


/// Lists all entries in the ledger.
fn list() -> Result<()>
{
    let ledger = load()?;

    for (_, value) in &ledger
    {
        println!("{: <14?}\t{: <14?}\t{: <20}\t{: <14}", value.freq, value.add_type, value.name, value.amount);
    }

    Ok(())
}


/// Generates a report for a month, extrapolating the values specified in the ledger.
///
/// The report has three sections; a table, a "breakdown", and a "coverage" section.
///
/// The table is simply a table displaying all the information about each transaction
/// in the ledger, with costs projected onto one month. For example, a yearly income
/// of $24k would be displayed as `2000.00`.
///
/// The breakdown shows the total expenses by category. You can specify the category
/// of an expense using the `--category` option of `pfr add`. 
///
/// The coverage section shows how much money you need in each of your accounts
/// in order to cover the months expenses. You can specify the account that each
/// expense is drawn from using the `--account` option of `pfr add`.
fn report() -> Result<()>
{
    let ledger = load()?;

    println!("Monthly Report\n");
    println!("{:<20}{:<20}{:<12}{:<10}{:<8}", "INCOME", "EXPENDITURE", "VALUE", "CATEGORY", "ACCOUNT");
    println!("-----------------------------------------------------------------------");

    let mut total: i64 = 0;
    let mut breakdown: HashMap<String, u64> = HashMap::new();
    let mut other_expenses = 0;

    let mut coverage: HashMap<String, u64> = HashMap::new();
    let mut other_alloc = 0;

    for (_, transaction) in &ledger
    {
        let mut income = String::new();
        let mut expend = String::new();
        let mut amount = String::new();
        let mut cat    = transaction.category.clone().unwrap_or(String::new());
        let mut accnt  = transaction.account.clone().unwrap_or(String::new());

        let multiplier: f32 = match transaction.freq
        {
            Frequency::daily => 30.0,
            Frequency::wkly  => 4.28, // note: extrapolating out to 30 day month means 4.28 weeks.
            Frequency::mthly => 1.0,
            Frequency::qtrly => 1.0/3.0,
            Frequency::yrly  => 1.0/12.0,
        };

        let money = Money { cents: (multiplier * transaction.amount.cents as f32) as u64 };
        amount = money.to_string();

        match transaction.add_type
        {
            AddType::income =>
            {
                income = transaction.name.clone();
                amount = format!(" {} ", amount);
                total += money.cents as i64;
            },

            AddType::expense =>
            {
                expend = transaction.name.clone();
                amount = format!("({})", amount);
                total -= money.cents as i64;

                match transaction.category
                {
                    Some(ref s) =>
                    {
                        let entry = breakdown.entry(s.clone()).or_insert(0);
                        *entry += money.cents;
                    },

                    None => other_expenses += money.cents,
                }

                match transaction.account
                {
                    Some(ref s) =>
                    {
                        let entry = coverage.entry(s.clone()).or_insert(0);
                        *entry += money.cents;
                    },

                    None => other_alloc += money.cents,
                }
            }
        }

        println!("{:<20}{:<20}{:<12}{:<10}{:<8}", income, expend, amount, cat, accnt);
    }

    println!("-----------------------------------------------------------------------");

    let total_str = if total > 0
    {
        format!(" {} ", Money { cents: total as u64 }.to_string())
    }
    else
    {
        let total = -total;
        format!("({})", Money { cents: total as u64 }.to_string())
    };

    println!("{:<20}{:<20}{:<12}{:<10}{:<8}\n", "", "TOTAL: ", total_str, "", "");

    println!("Breakdown:");
    for (name, value) in &breakdown
    {
        println!("{:<16}{:10}", name, Money{ cents: *value });
    }

    println!("{:<16}{:<10}\n", "(other)", Money{ cents: other_expenses });

    println!("Coverage:");
    for (name, value) in &coverage
    {
        println!("{:<10} -> {:<10}", Money{ cents: *value }, name);
    }

    println!("{:<10}    {:<10}", Money{ cents: other_alloc }, "(unallocated)");

    Ok(())
}


