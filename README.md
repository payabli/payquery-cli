# payquery

## Dependencies
- [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
- [git](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git)

## Setup

1. Clone the repo 
```bash
git clone https://github.com/payabli/payquery-cli
```

2. Compile the code 
```bash
cd payquery-cli; cargo build --release
```

3. Copy the binary to your PATH
```bash
cp target/release/payquery <dir-in-your-PATH>
```

```
payquery - A command-line interface for calling Payabli's Query APIs.

USAGE:
payquery [SUBCOMMAND] [OPTIONS] [only N] API endpoint [CLAUSES]

SUBCOMMANDS:
new                   Create a new configuration.
list                  List all available configurations.
help                  Show this help message.

OPTIONS:
--json                Output in JSON format (default).
--yaml                Output in YAML format.
--quiet               Don't output information besides the query result.

CLAUSES:
only N ...            Limit the number of records to N.
(comes before the API endpoint)
... for NAME          Use the configuration named NAME.
... where FILTERS     FILTER records based on the given conditions.
(https://docs.payabli.com/developer-guides/reporting-filters-and-conditions-reference)
... by FIELD          Sort records by FIELD in ascending order.
... by FIELD desc     Sort records by FIELD in descending order.
... crop              Output only the sorted field values.
(must come after a BY clause)

EXAMPLES:
payquery new
payquery list
payquery only 5 transactions
payquery chargebacks where method eq card
payquery batches for ISV_Pizzabli by TransactionDate
payquery only 10 customers for ISV_Pizzabli where firstname eq John by Lastname crop

CONFIGURATION:
Configurations are stored in a YAML file located in your home directory as 'payquery.yml'.
Each configuration contains API token, organization ID, entrypoint, and environment.
Use the 'new' subcommand to create or update configurations.
```
