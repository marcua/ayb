# `ayb`
`ayb` makes it easy to create databases, share them with collaborators, and query them from anywhere.

With `ayb`, all your (data)base can finally belong to you. Move SQL for great justice.

Here's a video of the web-based user interface that comes packaged with `ayb`

https://github.com/user-attachments/assets/2147dde7-21c5-4aa1-8733-a0f8d3ba0642

[![Build status](https://github.com/marcua/ayb/actions/workflows/tests.yml/badge.svg)](https://github.com/marcua/ayb/actions/workflows/tests.yml)

## Introduction

`ayb` is a database management system with easy-to-host instances that enable users to quickly register an account, create databases, share them with collaborators, and query them from a web application or the command line. An `ayb` server allows users to create SQLite databases (other databases to come), and then exposes those databases through an HTTP API.

To learn more about why `ayb` matters, how it works, or who it's for, [read this introductory blog post](https://blog.marcua.net/2023/06/25/ayb-a-multi-tenant-database-that-helps-you-own-your-data.html).

*alpha warning*: `ayb` is neither feature complete nor production-ready. Functionality like authentication, permissions, collaboration, isolation, high availability, and transaction support are on the [Roadmap](#roadmap) but not available today. I work on `ayb` as a hobbyist side project.

## Getting started

### Installing
`ayb` is written in Rust, and is available as the `ayb` crate. Assuming you have [installed Rust on your machine](https://www.rust-lang.org/tools/install), installing `ayb` takes a single command:

```bash
cargo install ayb
```

Alternatively, you can run `ayb` using Docker - see the [Docker section](#docker) for details.

### Running a server
An `ayb` server stores its metadata in [SQLite](https://www.sqlite.org/index.html) or [PostgreSQL](https://www.postgresql.org/), and stores the databases it's hosting on a local disk. An `ayb.toml` file tells the server what host/port to listen for connections on, how to connect to the database, and the data path for the hosted databases. You can generate a starter file with `ayb default_server_config`.

```bash
$ ayb default_server_config > ayb.toml

$ cat ayb.toml

host = "0.0.0.0"
port = 5433
# If hosting publicly, this URL prefix will be used to create public URLs for your instance:
# public_url = "https://ayb.example.com"
database_url = "sqlite://ayb_data/ayb.sqlite"
# Or, for Postgres:
# database_url = "postgresql://postgres_user:test@localhost:5432/test_db"
data_path = "./ayb_data"

[authentication]
# A secret (and unique to your server) key that is used for account registration.
fernet_key = "<UNIQUE_KEY_GENERATED_BY_COMMAND>="
token_expiration_seconds = 3600

[email.smtp]
from = "Server Sender <server@example.org>"
reply_to = "Server Reply <replyto@example.org>"
smtp_host = "localhost"
smtp_port = 465
smtp_username = "login@example.org"
smtp_password = "the_password"

[email.file]
path = "./ayb_data/emails.jsonl"

[web]
hosting_method = "Local"

[cors]
origin = "*"
```

Any setting in `ayb.toml` can be overridden using environment variables with the `AYB_` prefix. Use `__` (double underscore) to separate nested fields (e.g., `AYB_PORT=8080`, `AYB_AUTHENTICATION__FERNET_KEY=...`, `AYB_EMAIL__SMTP__HOST=...`).

Running the server then requires one command
```bash
$ ayb server
```


### Running a client
Once the server is running, you can register a user (in this case, `marcua`), create a database `marcua/test.sqlite`, and issue SQL as you like. Here's how to do that at the command line:

```bash
$ ayb client --url http://127.0.0.1:5433 register marcua you@example.com
Check your email to finish registering marcua

# You will receive an email at you@example.com instructing you to type the next command
$ ayb client confirm <TOKEN_FROM_EMAIL>
Successfully authenticated and saved token <API_TOKEN>

$ ayb client create_database marcua/test.sqlite
Successfully created marcua/test.sqlite

$ ayb client list marcua
 Database slug | Type
---------------+--------
 test.sqlite   | sqlite

$ ayb client query marcua/test.sqlite "CREATE TABLE favorite_databases(name varchar, score integer);"

Rows: 0

# If you don't pass a query to the query command, ayb launches an interactive query session
$ ayb client query marcua/test.sqlite
Launching an interactive session for marcua/test.sqlite
marcua/test.sqlite> INSERT INTO favorite_databases (name, score) VALUES ("PostgreSQL", 10);

Rows: 0
marcua/test.sqlite> INSERT INTO favorite_databases (name, score) VALUES ("SQLite", 9);

Rows: 0
marcua/test.sqlite> INSERT INTO favorite_databases (name, score) VALUES ("DuckDB", 9);

Rows: 0
marcua/test.sqlite> SELECT * FROM favorite_databases;
 name       | score
------------+-------
 PostgreSQL | 10
 SQLite     | 9
 DuckDB     | 9

Rows: 3
marcua/test.sqlite>

$ ayb client update_profile marcua --display_name 'Adam Marcus' --links 'http://marcua.net'

Successfully updated profile

$ ayb client profile marcua
 Display name | Description | Organization | Location | Links
--------------+-------------+--------------+----------+-------------------
 Adam Marcus  |             |              |          | http://marcua.net
```

Note that the command line also saved a configuration file for your
convenience so you don't have to keep entering a server URL or API
token. If you ever want to set these explicitly, the `--url`/`--token`
command-line flags and `AYB_SERVER_URL`/`AYB_API_TOKEN` environment
variables will override whatever is in the saved configuration. By
default, the configuration file can be found in:
* Linux: `/home/alice/.config/ayb/ayb.json`
* MacOS (untested): `/Users/Alice/Library/Application Support/org.ayb.ayb/ayb.json`
* Windows (untested): `C:\Users\Alice\AppData\Roaming\ayb\ayb\config\ayb.json`

The command line invocations above are a thin wrapper around `ayb`'s HTTP API. Here are the same commands as above, but with `curl`:
```bash
$ curl -w "\n" -X POST http://127.0.0.1:5433/v1/register -H "entity-type: user" -H "entity: marcua" -H "email-address: your@example.com"

{}

$ curl -w "\n" -X POST http://127.0.0.1:5433/v1/confirm -H "authentication-token: TOKEN_FROM_EMAIL"

{"entity":"marcua","token":"<API_TOKEN>"}

$ curl -w "\n" -X POST http://127.0.0.1:5433/v1/marcua/test.sqlite/create -H "db-type: sqlite" -H "authorization: Bearer <API_TOKEN_FROM_PREVIOUS_COMMAND>"

{"entity":"marcua","database":"test.sqlite","database_type":"sqlite"}

$ curl -w "\n" -X PATCH http://127.0.0.1:5433/v1/entity/marcua -H "authorization: Bearer <API_TOKEN_FROM_PREVIOUS_COMMAND>" -d "{\"display_name\": \"Adam Marcus\"}"

{}

$ curl -w "\n" -X GET http://localhost:5433/v1/entity/marcua -H "authorization: Bearer <API_TOKEN_FROM_PREVIOUS_COMMAND>"

{"slug":"marcua","databases":[{"slug":"test.sqlite","database_type":"sqlite"}],"profile":{"display_name":"Adam Marcus"}}

$ curl -w "\n" -X POST http://127.0.0.1:5433/v1/marcua/test.sqlite/query -H "authorization: Bearer <API_TOKEN_FROM_PREVIOUS_COMMAND>" -d 'CREATE TABLE favorite_databases(name varchar, score integer);'

{"fields":[],"rows":[]}

$ curl -w "\n" -X POST http://127.0.0.1:5433/v1/marcua/test.sqlite/query -H "authorization: Bearer <API_TOKEN_FROM_PREVIOUS_COMMAND>" -d "INSERT INTO favorite_databases (name, score) VALUES (\"PostgreSQL\", 10);"

{"fields":[],"rows":[]}

$ curl -w "\n" -X POST http://127.0.0.1:5433/v1/marcua/test.sqlite/query -H "authorization: Bearer <API_TOKEN_FROM_PREVIOUS_COMMAND>" -d "INSERT INTO favorite_databases (name, score) VALUES (\"SQLite\", 9);"

{"fields":[],"rows":[]}

$ curl -w "\n" -X POST http://127.0.0.1:5433/v1/marcua/test.sqlite/query -H "authorization: Bearer <API_TOKEN_FROM_PREVIOUS_COMMAND>" -d "INSERT INTO favorite_databases (name, score) VALUES (\"DuckDB\", 9);"

{"fields":[],"rows":[]}

$ curl -w "\n" -X POST http://127.0.0.1:5433/v1/marcua/test.sqlite/query -H "authorization: Bearer <API_TOKEN_FROM_PREVIOUS_COMMAND>" -d "SELECT * FROM favorite_databases;"

{"fields":["name","score"],"rows":[["PostgreSQL","10"],["SQLite","9"],["DuckDB","9"]]}
```

### Web interface
`ayb` comes with a fully functional web interface. With the server configuration shown above, visit [http://localhost:5433/register](http://localhost:5433/register) to get started. The web interface allows you to register, log in, create databases, and run queries through your browser without needing to use the command line client.

The default configuration (with `web.hosting_method` set to `Local`) enables it automatically, though you can remove the `web` section from your configuration if you only want an API server.

### Email Configuration

`ayb` supports multiple email backends for sending registration and login emails. A standard SMTP configuration can be used in production settings, and a file-based log can also be configured to help with development and testing. At least one of the backends must be configured for `ayb` to start.

#### SMTP email backend
For production deployments, configure SMTP to send emails through your email provider:

```toml
[email.smtp]
from = "Your App <app@example.com>"
reply_to = "Support <support@example.com>"
smtp_host = "smtp.example.com"
smtp_port = 587
smtp_username = "your_username"
smtp_password = "your_password"
```

#### Local file email backend (development/testing)
For development or testing, you can write emails to a local file instead, where each email is JSON-encoded with one JSON-encoded email per line:

```toml
[email.file]
path = "/path/to/emails.jsonl"
```

### Snapshots / backups

You can configure `ayb` to periodically upload snapshots of each
database to [S3](https://aws.amazon.com/s3/)-compatible storage to
recover from the failure of the machine running `ayb` or revert to a
previous copy of the data. Each snapshot is compressed (using
[zstd](https://facebook.github.io/zstd/)) and only uploaded if the database changed
since the last snapshot. To enable snapshot-based backups, include a
configuration block like the following in your `ayb.toml`:

```toml
[snapshots]
sqlite_method = "Vacuum"
access_key_id = "YOUR_S3_ACCESS_KEY_ID"
secret_access_key = "YOUR_S3_ACCESS_KEY_SECRET"
bucket = "bucket-to-upload-snapshots"
path_prefix = "some/optional/prefix"
endpoint_url = "https://url-endpoint-of-s3-compatible-provider.com"  # Optional
region = "us-east-1"  # Optional
force_path_style = false  # Optional

[snapshots.automation]
interval = "10m"
max_snapshots = 3
```

Here is an explanation of the parameters:
* `sqlite_method`: The two SQLite backup methods are [Vacuum](https://www.sqlite.org/lang_vacuum.html#vacuuminto) and [Backup](https://www.sqlite.org/backup.html). `ayb` only supports `Vacuum` for now.
* `access_key_id` / `secret_access_key`: The access key ID and secret to upload/list snapshots to your S3-compatible storage provider.
* `bucket`: The name of the bucket to which to upload snapshots.
* `bucket_prefix`: (Can be blank) if you want to upload snapshots to a prefixed path inside `bucket` (e.g., `my-bucket/the-snapshots`), provide a prefix (e.g., `the-snapshots`).
* `endpoint_url`: (Optional if using AWS S3) Each S3-compatible storage provider will tell you their own endpoint to manage your buckets.
* `region`: (Optional if using AWS S3) Some S3-compatible storage providers will request a region in their network where your bucket will live.
* `force_path_style`: (Optional, legacy) If included and `true`, will use the legacy [path-style](https://docs.aws.amazon.com/AmazonS3/latest/userguide/VirtualHosting.html#path-style-access) method of referencing buckets. Used in `ayb`'s end-to-end tests and might be helpful beyond, but start without it.
* `interval`: How frequently to take a snapshot of your data in human-readable format (e.g., every 30 minutes = `30m`, every hour = `1h`, every hour and 30 minutes = `1h30m`, with [more examples here](https://docs.rs/go-parse-duration/latest/go_parse_duration/)).
* `max_snapshots`: How many old snapshots to keep before pruning the oldest ones.

Once snapshots are enabled, you will see logs on the server with each periodic snapshot run. The following example shows how snapshots work, including how to list and restore them (using `interval = "3s"` and `max_snapshots = 2`):

```bash
$ ayb client create_database marcua/snapshots.sqlite
Successfully created marcua/snapshots.sqlite

$ ayb client query marcua/snapshots.sqlite "CREATE TABLE favorite_databases(name varchar, score integer);"
Rows: 0

$ ayb client query marcua/snapshots.sqlite "INSERT INTO favorite_databases (name, score) VALUES (\"PostgreSQL\", 10);"
Rows: 0

# Wait longer than 3 seconds before inserting the next row, so that a snapshot with just PostgreSQL exists.
$ ayb client query marcua/snapshots.sqlite "INSERT INTO favorite_databases (name, score) VALUES (\"SQLite\", 9);"
Rows: 0

$ ayb client query marcua/snapshots.sqlite "SELECT * FROM favorite_databases;"
 name       | score
------------+-------
 PostgreSQL | 10
 SQLite     | 9

Rows: 2

# Wait longer than 3 seconds before listing snapshots to ensure that a snapshot with SQLite exists as well.
$ ayb client list_snapshots marcua/snapshots.sqlite
 Name                                                             | Last modified
------------------------------------------------------------------+---------------------------
 f9e01a396fb7f91be988c26d43f9ffa667bd0fd05009b231aa61ea1073d34423 | 2024-08-18T15:05:04+00:00
 856e21f7cae8383426cd2e0599caf6e83962b051af4734ab5c53aff87ea0ff45 | 2024-08-18T15:04:40+00:00

# Restore the older snapshot, which didn't contain SQLite
$ ayb client restore_snapshot marcua/snapshots.sqlite 856e21f7cae8383426cd2e0599caf6e83962b051af4734ab5c53aff87ea0ff45
Restored marcua/snapshots.sqlite to snapshot 856e21f7cae8383426cd2e0599caf6e83962b051af4734ab5c53aff87ea0ff45

$ ayb client query marcua/snapshots.sqlite "SELECT * FROM favorite_databases;"
 name       | score
------------+-------
 PostgreSQL | 10

Rows: 1
```

Credits: the design of snapshot-based backups was influenced by that
of
[rqlite](https://rqlite.io/docs/guides/backup/#automatic-backups). Thank
you to the authors for their great design and documentation.

### Permissions

By default, only the owner / creator of an `ayb` database can access
it. It's possible to share `ayb` databases in two ways:
* By setting the public sharing level of the database to give all entities some level of access to the database.
* By sharing the database with a particular entity.

To set the public sharing level of a database, select one of the following options:
```bash
# The default setting: no entity will be able to access the database
# unless they specifically get permissions.
$ ayb client update_database marcua/test.sqlite --public-sharing-level no-access

# With a public sharing level of `fork`, entities will be able to see
# the database in the owner's list of databases using `ayb client
# list` and fork a copy of the database under their own account. They
# won't be able to query the database unless they fork it. Note:
# Listing access is implemented today, but forking one database into
# another account is not yet implemented.
$ ayb client update_database marcua/test.sqlite --public-sharing-level fork

# In addition to the listing and forking access that `fork`
# allows, `read-only` access allows any entity to
# issue a read-only (e.g., SELECT) query against the database. They
# can't modify the database.
$ ayb client update_database marcua/test.sqlite --public-sharing-level read-only
```

To provide a specific user with access to a database, select one of the following:
```bash
# Revoke access to a database from an entity.
$ ayb client share marcua/test.sqlite sofia no-access

# Allow an entity to make read-only (e.g., SELECT) queries against a
# database.
$ ayb client share marcua/test.sqlite sofia read-only

# Allow an entity to make any type of query against a database.
$ ayb client share marcua/test.sqlite sofia read-write

# Allow an entity to not only modify a database, but also to manage
# snapshots and change the permissions of any non-owner entity.
$ ayb client share marcua/test.sqlite sofia manager

# List all entities that have access to a database.
$ ayb client list_database_permissions marcua/test.sqlite
```

### Isolation
`ayb` allows multiple users to run queries against databases that are
stored on the same machine. Isolation enables you to prevent one user
from accessing another user's data, and allows you to restrict the
resources any one user is able to utilize.

By default, `ayb` uses
[SQLITE_DBCONFIG_DEFENSIVE](https://www.sqlite.org/c3ref/c_dbconfig_defensive.html)
flag and sets
[SQLITE_LIMIT_ATTACHED](https://www.sqlite.org/c3ref/c_limit_attached.html#sqlitelimitattached)
to `0` in order to prevent users from corrupting the database or
attaching to other databases on the filesystem.

For further isolation, `ayb` can use [nsjail](https://nsjail.dev/)
(only when running on Linux) to isolate each query's filesystem access
and resources. When this form of isolation is enabled, `ayb` starts a
new `nsjail`-managed process to execute the query against the
database. We have not yet benchmarked the performance overhead of this
approach.

To enable this deeper form of isolation on Linux, you must first build
`nsjail`, which you can do through
[scripts/build_nsjail.sh](scripts/build_nsjail.sh). Note that `nsjail`
depends on a few other packages. If you run into issues building it,
it might be helpful to see its
[Dockerfile](https://github.com/google/nsjail/blob/master/Dockerfile)
to get a sense of those requirements.

Once you have a path to the
`nsjail` binary, add the following to your `ayb.toml`:

```toml
[isolation]
nsjail_path = "path/to/nsjail"
```

## Docker

On every release, a docker image is built and pushed to
`ghcr.io/marcua/ayb`. For now, docker images are available for
`linux-amd64`. If you would like a `linux-arm64` image, follow
and comment on
[this issue](https://github.com/marcua/ayb/issues/523).

To pull the latest version of the image:
```bash
docker pull ghcr.io/marcua/ayb
```

You can then create an alias for convenience:
```bash
alias ayb="docker run --network host ghcr.io/marcua/ayb ayb"
```

To run the server, you'll need to create an `ayb.toml` configuration
file (see [Running a server](#running-a-server)),
create a data directory for the databases, and map the configuration and
data directory as volumes when running the container. For example:
```bash
docker run -v $(pwd)/ayb.toml:/ayb.toml \
          -v $(pwd)/ayb_data:/ayb_data \
          -p 5433:5433 \
          ghcr.io/marcua/ayb \
          ayb server --config /ayb.toml
```

Then use the client as normal:
```bash
ayb client --url http://127.0.0.1:5433 register marcua you@example.com
```

## Testing
`ayb` is largely tested through [end-to-end
tests](tests/e2e.rs) that mimic as realistic an environment as
possible. Individual modules may also provide more specific unit
tests. To set up your environment for running end-to-end tests, type:

```bash
tests/set_up_e2e_env.sh
```

The Postgres-based tests require a `postgres_user` user with password `test`. Create this user with `createuser -P postgres_user` and enter `test` as the password when prompted. Then grant the user database creation privileges:
* On Linux (tested Ubuntu): `sudo -u postgres psql -c "alter user postgres_user createdb;"`
* On macOS: `createuser -s postgres` followed by `psql -U postgres -c "alter user postgres_user createdb;"`

After your environment is set up, you can run the tests with:

```bash
cargo test --verbose
```

In order to mimic as close to a realistic environment as possible, the end-to-end tests mock out very little functionality. The `tests/set_up_e2e_env.sh` script, which has been used extensively in Ubuntu, does the following:
* Sets up a Python virtual environment and installs requirements for various helpers.
* Installs the requirements for a [MinIO](https://min.io/) server and then runs that server in the background (requires Docker) in order to test database snapshotting functionality that stores snapshots in S3-compatible storage.
* Installs an `nsjail` binary to test `ayb`'s [isolation](#isolation) functionality.

## FAQ

### Who is `ayb` for?
The introductory blog post has [a section describing each group that stands to benefit](https://blog.marcua.net/2023/06/25/ayb-a-multi-tenant-database-that-helps-you-own-your-data.html#students-sharers-and-sovereigns) from `ayb`'s aim to make it easier to create a database, interact with it, and share it with relevant people/organizations. Students would benefit from encountering less operational impediments to writing their first SQL query or sharing their in-progress database with a mentor or teacher for help. Sharers like scientists and journalists would benefit from an easy way to post a dataset and share it with collaborators. Finally, anyone concerned about the sovereignty of their data would benefit from a world where it's so easy to spin up a database that more of their data can live in databases they control.

### What's with the name?
Thank you for asking. [I hope the answer elicits some nostalgia](https://www.youtube.com/watch?v=qItugh-fFgg)! Shout out to Meelap Shah and Eugene Wu for convincing me to not call this project `stacks`, to Andrew Lange-Abramowitz for making the connection to the storied meme, and to Meredith Blumenstock for listening to me fret over it all.

## Roadmap
Here's a rough roadmap for the project, with items near the top of the list more likely to be completed first. The nitty-gritty list of prioritized issues can be found on [this project board](https://github.com/marcua/ayb/projects/1), with the most-likely-to-be-completed issues near the top of the to-do list.

* Make the single-user `ayb` experience excellent
  * [x] Reduce reliance on PostgreSQL (SQLite metadata storage). Given that the goal of `ayb` is to make it easier to create, share, and query databases, it's frustrating that running `ayb` requires you to pay the nontrivial cost of operationalizing PostgreSQL. While Postgres will be helpful for eventually coordinating between multiple `ayb` nodes, a single-node version should be able to store its metadata in SQLite with little setup costs.
  * [x] Authentication and permissions. Add authentication/the ability to log in, and add permissions to endpoints so that you can't just issue queries against any database.
  * [x] Isolation. Since an `ayb` instance can have multiple tenants/databases, we want to use one of the many container/isolate/microVM projects to ensure that one tenant isn't able to access another tenant's data.
  * [x] Persistence beyond the node. Back databases up to persistent S3-compatible storage and allow (for now) manual recovery on failure.
  * [ ] Clustering. Support for multiple `ayb` nodes to serve databases and requests. Whereas a single database will not span multiple machines, parallelism/distribution will happen across users and databases.
  * [ ] Sessions/transactions. `ayb`'s query API is a stateless request/response API, making it impossible to start a database transaction or issue multiple queries in a session. Exposing sessions in the API will allow multiple statements per session, and by extension, transactions.
  * [ ] Import/export of databases. `ayb` already uses existing well-established file formats (e.g., SQLite). There should be endpoints to import existing databases into `ayb` in those formats or export the underlying files so you're not locked in.
  * [ ] High availablity/automatic failover. While `ayb` provides snapshot-based backups to protect against cataclysmic failures, the recovery process is manual. Streaming databases to replicas and switching to replicas on failure will make `ayb` more highly available.
* Extend `ayb` to more people and software
  * [x] Collaboration. In addition to making it easy to create and query databases, it should be easy to share databases with others. Two use cases include adding private collaborators and allowing public read-only access.
  * [ ] Forking. Allowing a user to fork their own copy of a database will enable collaborators to remix and build on each others' work.
  * [ ] Versioning. To both make it less scary to execute sensitive operations and to make it possible for scientists to reference and publish checkpoints of their work, a user should be able to snapshot and revert to a database at a point in time.
  * [ ] DuckDB. Allowing users to create a DuckDB database in addition to a SQLite database would allow you to create a data warehouse with a single command. This effort is dependent on the DuckDB project. First, the DuckDB file format is rapidly changing ahead of the project's 1.0 release. Additionally, I don't know of an equivalent streaming replication project to LiteFS for DuckDB that handles *persistence beyond the node*.
  * [ ] PostgreSQL wire protocol. While an HTTP API makes it easy to build new web apps, exposing `ayb` over the PostgreSQL wire protocol will allow existing tools and libraries to connect to and query an `ayb` database.
* Increase discoverability with a web frontend
  * [x] Provide a web interface analogous to the command line interface. Much like GitHub/Gitea/Forgejo make git more approachable, you shouldn't have to pay a command line knowledge tax in order to create, share, and query an `ayb` database.
  * [ ] Explore people's public datasets. Beyond simplifying the command line, platforms like GitHub also make it easier to find a user's publicly shared repositories, follow along in their work, and fork a copy for your own exploration. That same experience should be possible for `ayb`-hosted databases.

## Contributing
(This section is inspired by the [LiteFS project](https://github.com/superfly/litefs#contributing), and is just one of the many things to love about that project.)

`ayb` contributions work a little different than most GitHub projects:
* If you have a small bug fix or typo fix, please PR directly to this repository.
* If you want to contribute documentation, please PR directly to this repository.
* If you would like to contribute a feature, create and discuss the feature in an issue on this GitHub repository first. Once the feature and some of its finer details are hashed out in the issue and potentially a design document, submit a pull request. I might politely decline pull requests that haven't first been discussed/designed.

This project has a roadmap and features are added and tested in a certain order. I'm adding a little friction in requiring a discussion/design document for features before submitting a pull request to ensure that I can focus my attention on well-motivated, well-sequenced, and well-understood functionality.

## Notes on releasing
As we are in the early days of `ayb`, we largely do patch releases (e.g., v0.1.7 -> v0.1.8). We use [cargo-release](https://crates.io/crates/cargo-release) for this.

To install `cargo-release`, run `cargo install cargo-release`.

To perform a patch release, ensure you are on `main` and run `cargo release patch`.
