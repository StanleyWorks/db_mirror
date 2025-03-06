## DB Sync
> A simple too that I use to copy production DB to my local setup.

### Please note 
This tool is still under heavy development and I cannot guarantee stability.
I also have a Node.js version [here](https://github.com/StanleyWorks/db_mirror_node)

### Setup 
This tool is configures with a TOML file. 
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

### More docs coming soon.
