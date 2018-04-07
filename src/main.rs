#![allow(non_camel_case_types)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
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

    /// save the current ledger using `name`; can be loaded again with `load name`.
    save { name: String },

    /// loads the ledger that was saved by `save name`.
    load{ name: String },

    /// backs up the current ledger
    backup,

    /// restores the backup
    restore,
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
        workdays,
        weekly,
        monthly,
        quarterly,
        yearly
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
        Commands::save { name }    => save(name),
        Commands::load { name }    => load(name),
        Commands::backup           => backup(),
        Commands::restore          => restore(),
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
    DuringInitialisation(std::io::Error),
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
        DuringInitialisation(e)             => println!(" while attempting to initialise: {}", e),
        DuringSerialisation(e)              => println!(" while attempting to save to the data file: {}", e),
        DuringDeSerialisation(e)            => println!(" while attempting to load from the data file: {}", e),
        CouldNotFindHomeDirectory           => println!(" while attempting to find the current user's home directory; couldn't find it"),
        NameIsAlreadyTaken(s)               => println!(": a transaction called {} is already present in the ledger", s)
    }

    return None;
}


/// `Ledger`, just an alias for a hashmap.
type Ledger = HashMap<String, Transaction>;


/// gets path for file called `name`, located in `~/.pfr/`
fn get_path(name: &str) -> Result<PathBuf>
{                     
    let mut home_dir = env::home_dir().ok_or_else(|| Error::CouldNotFindHomeDirectory)?;
    home_dir.push(".pfr/");
    home_dir.push(name);

    return Ok(home_dir);
}


/// Saves the ledger to the pfr data file.
fn save_ledger(name: &str, ledger: Ledger) -> Result<()>
{
    let ledgerfile = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(get_path(name)?)
        .map_err(|e| Error::WhileAttemptingToOpenDataFile(e))?;

    serde_json::to_writer_pretty(ledgerfile, &ledger)
        .map_err(|e| Error::DuringSerialisation(e))
}


/// loads ledger from file
fn load_ledger(name: &str) -> Result<Ledger>
{
    let ledgerfile = OpenOptions::new()
        .read(true)
        .open(get_path(name)?)
        .map_err(|e| Error::WhileAttemptingToOpenDataFile(e))?;

    serde_json::from_reader(ledgerfile)
        .map_err(|e| Error::DuringDeSerialisation(e))
}


/// saves the ledger to the current ledgerfile.
fn save_current_ledger(ledger: Ledger) -> Result<()>
{
    save_ledger(".current_data", ledger)
}


/// loads the current ledger
fn load_current_ledger() -> Result<Ledger>
{
    load_ledger(".current_data")
}


/// clears the current ledger
fn init() -> Result<()>
{
    let mut home_dir = env::home_dir().ok_or_else(|| Error::CouldNotFindHomeDirectory)?;
    home_dir.push(".pfr/");

    if !home_dir.exists()
    {
        fs::create_dir(home_dir)
            .map_err(|e| Error::DuringInitialisation(e))?;
    }

    save_current_ledger(Ledger::new())
}


/// Adds a new entry to the ledger.
/// Errors if an entry with the given name already exists.
fn add(ac: Transaction) -> Result<()>
{
    let mut ledger = load_current_ledger()?;

    return match ledger.insert(ac.name.clone(), ac)
    {
        Some(val) =>
        {
            let e = Error::NameIsAlreadyTaken(val.name.clone());
            ledger.insert(val.name.clone(), val);
            Err(e)
        },
        
        None => save_current_ledger(ledger),
    }
}


/// Removes an entry from the ledger.
fn rm(rc: RmCommand) -> Result<()>
{
    let mut ledger = load_current_ledger()?;
    ledger.remove(&rc.name);
    save_current_ledger(ledger)
}


/// Lists all entries in the ledger.
fn list() -> Result<()>
{
    let ledger = load_current_ledger()?;

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
    let ledger = load_current_ledger()?;

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
            Frequency::daily     => 30.0,
            Frequency::weekly    => 4.28, // note: extrapolating out to 30 day month means 4.28 weeks.
            Frequency::workdays  => 21.4, // note: 4.28 weeks * 5 day weeks
            Frequency::monthly   => 1.0,
            Frequency::quarterly => 1.0/3.0,
            Frequency::yearly    => 1.0/12.0,
        };

        let money = Money { cents: (multiplier * transaction.amount.cents as f32) as u64 };
        amount.push_str(&money.to_string());

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


/// changes the current ledger to be the one called `name`
fn load(name: String) -> Result<()>
{
    save_ledger(".current_data", load_ledger(&name)?)
}


/// saves the current ledger to file as `name`
fn save(name: String) -> Result<()>
{
    save_ledger(&name, load_current_ledger()?)
}


/// saves a backup
fn backup() -> Result<()>
{
    save(".current_backup".to_string())
}


/// restores the backup
fn restore() -> Result<()>
{
    load(".current_backup".to_string())
}

