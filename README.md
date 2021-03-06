# `pfr`: personal financial reporter

`pfr` is a command-line tool for helping to manage personal finances.

I like to allocate my money on a monthly basis; at the end of each month, I 
load enough money onto my card to cover the expected expenses for the month.

A small hiccup with this approach is that monthly isn't always the easiest way
to think about some recurring transactions in my life. For example, I'd like to
allocate $40 per week on food but $100 per month on petrol. These expenses can
also change slightly from week to week or month to month. Also, some expenses
come from different accounts, so I need to keep track of that too.

Because of this, I need to sit down and work out how much is coming and going
manually, as well as split things by which account they are coming out of, but
for all of my expenses. This sucks. So I wrote a program to do it for me.

[visit me on crates.io](https://crates.io/crates/pfr)


# Usage:

You can tell `pfr` what your incomes and expenses are, and how often they occur.

```bash
# I make $800.00 a month from work
pfr add income monthly work 800 

# I spend $40.00 a week on food, and this comes
# out of my direct-debit account
pfr add expense weekly food 40 --account "direct debit"

# Insurance for my car costs $20.00 a month, and
# is paid from my automatic payments account
pfr add expense weekly "car insurance" 20 --account automatic --category car

# I also spend $60 on petrol per week, but this
# comes from my EFTPOS (direct-debit) card.
pfr add expense weekly petrol 60 --account "direct debit" --category car

```

You can list the transactions that `pfr` knows about using `pfr list`.

```bash
$ pfr list
monthly	income	work                	 800.00
weekly	expense	petrol              	  60.00
weekly	expense	food                	  40.00
monthly	expense	car insurance       	  20.00
```

Finally, you can also generate a report:

```bash
$ pfr report
Monthly Report

# This table shows all of your incomes and expenditures,
# extrapolated to 1-month (30 days). Negative values are enclosed in (parentheses).

# This gives you an overview of all the transactions.

INCOME              EXPENDITURE         VALUE       CATEGORY  ACCOUNT 
-----------------------------------------------------------------------
work                                      800.00                      
                    car insurance       (  20.00)   car       automatic
                    petrol              ( 256.80)   car       direct debit
                    food                ( 171.20)             direct debit
-----------------------------------------------------------------------
                    TOTAL:                352.00                      


# This table shows your expenses, broken down by category.

Breakdown:
car              276.80
(other)          171.20


# This table shows the amount of money I need to put in each
# account in order to cover my expenses.

Coverage:
 428.00 -> direct debit
  20.00 -> automatic 
   0.00    (unallocated)
```

# Installing

Via `cargo`:

```bash
cargo install pfr
pfr init # initialises the storage files, in ~/.pfr/
```


# Saving, Loading and Backups

(New in `v0.2.0`)

`pfr save <name>` will save a snapshot of the current ledger as `name`. You can
reset back to that state with `pfr load name`.

`pfr backup` does the same, but does not require you to specify a name. You can
reset back to state with `pfr restore`. This can be handy if you just want to see
the result of some small changes, without keeping them permanently.


# Updating from `v0.1.1` to `v0.2.0`

`pfr` now stores it's data in the directory `~/.pfr/`. After updating the tool
(by `cargo install --force`) and re-running `pfr init`, you should `mv
~/.pfr_data ~/.pfr/.current_data`.

Also note that the frequency specifiers have been expanded; e.g `wkly -> weekly`.


# Data format

`pfr` stores all of your transactions in JSON format in `~/.pfr/`.

