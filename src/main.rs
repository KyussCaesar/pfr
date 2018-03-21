#![allow(non_camel_case_types)]

use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::env;

#[macro_use] extern crate structopt;
#[macro_use] extern crate clap;
#[macro_use] extern crate serde_derive;

extern crate serde;
extern crate serde_json;

use structopt::StructOpt;

arg_enum!
{
    #[derive(Debug, Serialize, Deserialize)]
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
    enum AddType
    {
        income,
        expense
    }
}

#[derive(StructOpt, Debug)]
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

    /// generate a report.
    report(ReportCommand),
}

#[derive(StructOpt, Serialize, Deserialize, Debug)]
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
    amount: f64,
}

#[derive(StructOpt, Debug)]
struct RmCommand
{
    /// the name of the entry to be removed.
    name: String,
}

#[derive(StructOpt, Debug)]
struct ReportCommand {}

fn main()
{
    let errors = match Commands::from_args()
    {
        Commands::init       => init(),
        Commands::add(ac)    => add(ac),
        Commands::rm(rc)     => rm(rc),
        Commands::list       => list(),
        Commands::report(rc) => report(rc),
    };

    // report error if there was one.
    errors.err().and_then(report_error);
}

/// Error enum
///
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

type Result<T> = std::result::Result<T, Error>;

/// Tries to open the pfr data file, located in the user's home directory.
///
/// If successful, returns `Ok(file_handle)`, otherwise, returns a variant of the
/// Error type describing the error that occurred.
fn get_file_handle() -> Result<File>
{
    let mut home_dir = env::home_dir().ok_or_else(|| Error::CouldNotFindHomeDirectory)?;

    home_dir.push(".pfr_data");

    return OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(home_dir)
        .map_err(|e| Error::WhileAttemptingToOpenDataFile(e));
}

/// Loads the ledger from the pfr data file.
fn load() -> Result<Ledger>
{
    serde_json::from_reader(get_file_handle()?).map_err(|e| Error::DuringDeSerialisation(e))
}

/// Saves the ledger to the pfr data file.
fn save(ledger: Ledger) -> Result<()>
{
    serde_json::to_writer_pretty(get_file_handle()?, &ledger).map_err(|e| Error::DuringSerialisation(e))
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
        println!("{:?}\t{:?}\t{}\t{}", value.add_type, value.freq, value.name, value.amount);
    }

    Ok(())
}

/// (Not implemented) Generates a report.
fn report(rc: ReportCommand) -> Result<()>
{
    println!("report {:?}", rc);

    Ok(())
}

