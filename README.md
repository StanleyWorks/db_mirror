## DB Sync
> A simple tool that I use to copy production DB to my local setup.

### Please note 
This tool is still under heavy development and I cannot guarantee stability.  
I also have a Node.js version [here](https://github.com/StanleyWorks/db_mirror_node)

### Setup 
This tool is configured with a TOML file.

1. Create a `config.toml`
2. Copy the config structure below

```toml
[primary_db]
host = ""
port = 3306
user = ""
password = ""
schema = ""

[secondary_db]
host = "127.0.0.1"
port = 3306
user = ""
password = ""
schema = ""
```

### Logging

This tool uses env_logger for log output.

You can control verbosity using the RUST_LOG environment variable:
	•	Default (no env var): shows info, warn, error

1. Show only warnings and errors:
```shell
RUST_LOG=warn cargo run
```


2. Silence all logs:
```shell
RUST_LOG=off cargo run
```


3. Show debug logs (if used):
```shell
RUST_LOG=debug cargo run
```



4. Log output format:

```log
[2025-04-16 22:25:41]: App started
````

### More docs coming soon.
