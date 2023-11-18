# `ayb`
`ayb` makes it easy to create databases, share them with collaborators, and query them from a web application or the command line.

With `ayb`, all your (data)base can finally belong to you. Move SQL for great justice.

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

### Running a server
An `ayb` server stores its metadata in [SQLite](https://www.sqlite.org/index.html) or [PostgreSQL](https://www.postgresql.org/), and stores the databases it's hosting on a local disk. An `ayb.toml` file tells the server what host/port to listen for connections on, how to connect to the database, and the data path for the hosted databases. You can generate a starter file with `ayb default_server_config`.

```bash
$ ayb default_server_config > ayb.toml

$ cat ayb.toml

host = "0.0.0.0"
port = 5433
database_url = "sqlite://ayb_data/ayb.sqlite"
# Or, for Postgres:
# database_url = "postgresql://postgres_user:test@localhost:5432/test_db"
data_path = "./ayb_data"

[authentication]
# A secret (and unique to your server) key that is used for account registration.
fernet_key = "Wviqeo0r4EEsTGG3fE0KP5eMIhwgdykcnlaSJQuEe4o="
token_expiration_seconds = 3600

[email]
from = "Server Sender <server@example.org>"
reply_to = "Server Reply <replyto@example.org>"
smtp_host = "localhost"
smtp_port = 465
smtp_username = "login@example.org"
smtp_password = "the_password"
```

Running the server then requires one command
```bash
$ ayb server
```


### Running a client
Once the server is running, you can set its URL as an environment variable called `AYB_SERVER_URL`, register a user (in this case, `marcua`), create a database `marcua/test.sqlite`, and issue SQL as you like. Here's how to do that at the command line:

```bash
$ export AYB_SERVER_URL=http://127.0.0.1:5433

$ ayb client register marcua you@example.com
Check your email to finish registering marcua

# You will receive an email at you@example.com instructing you to type the next command
$ ayb client confirm <TOKEN_FROM_EMAIL>
Successfully authenticated and saved token <API_TOKEN>

$ export AYB_API_TOKEN=<API_TOKEN_FROM_PREVIOUS_COMMAND>

$ ayb client create_database marcua/test.sqlite
Successfully created marcua/test.sqlite

$ ayb client query marcua/test.sqlite "CREATE TABLE favorite_databases(name varchar, score integer);"

Rows: 0

$ ayb client query marcua/test.sqlite "INSERT INTO favorite_databases (name, score) VALUES (\"PostgreSQL\", 10);"

Rows: 0

$ ayb client query marcua/test.sqlite "INSERT INTO favorite_databases (name, score) VALUES (\"SQLite\", 9);"

Rows: 0

$ ayb client query marcua/test.sqlite "INSERT INTO favorite_databases (name, score) VALUES (\"DuckDB\", 9);"

Rows: 0

$ ayb client query marcua/test.sqlite "SELECT * FROM favorite_databases;"
 name       | score 
------------+-------
 PostgreSQL | 10 
 SQLite     | 9 
 DuckDB     | 9 

Rows: 3
```

The command line invocations above are a thin wrapper around `ayb`'s HTTP API. Here are the same commands as above, but with `curl`:
```bash
$ curl -w "\n" -X POST http://127.0.0.1:5433/v1/register -H "entity-type: user" -H "entity: marcua" -H "email-address: your@example.com"

{}

$ curl -w "\n" -X POST http://127.0.0.1:5433/v1/confirm -H "authentication-token: TOKEN_FROM_EMAIL"

{"token":"<API_TOKEN>"}

$ curl -w "\n" -X POST http://127.0.0.1:5433/v1/marcua/test.sqlite/create -H "db-type: sqlite" -H "authorization: Bearer <API_TOKEN_FROM_PREVIOUS_COMMAND>"

{"entity":"marcua","database":"test.sqlite","database_type":"sqlite"}

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

## FAQ

### Who is `ayb` for?
The introductory blog post has [a section describing each group that stands to benefit](https://blog.marcua.net/2023/06/25/ayb-a-multi-tenant-database-that-helps-you-own-your-data.html#students-sharers-and-sovereigns) from `ayb`'s aim to make it easier to create a database, interact with it, and share it with relevant people/organizations. Students would benefit from encountering less operational impediments to writing their first SQL query or sharing their in-progress database with a mentor or teacher for help. Sharers like scientists and journalists would benefit from an easy way to post a dataset and share it with collaborators. Finally, anyone concerned about the sovereignty of their data would benefit from a world where it's so easy to spin up a database that more of their data can live in databases they control.

### What's with the name?
Thank you for asking. [I hope the answer elicits some nostalgia](https://www.youtube.com/watch?v=qItugh-fFgg)! Shout out to Meelap Shah and Eugene Wu for convincing me to not call this project `stacks`, to Andrew Lange-Abramowitz for making the connection to the storied meme, and to Meredith Blumenstock for listening to me fret over it all.

## Roadmap
Here's a rough roadmap for the project, with items near the top of the list more likely to be completed first. The nitty-gritty list of prioritized issues can be found on [this project board](https://github.com/marcua/ayb/projects/1), with the most-likely-to-be-completed issues near the top of the to-do list.

* Make the single-user `ayb` experience excellent
  * [x] Reduce reliance on PostgreSQL (SQLite metadata storage). Given that the goal of `ayb` is to make it easier to create, share, and query databases, it's frustrating that running `ayb` requires you to pay the nontrivial cost of operationalizing PostgreSQL. While Postgres will be helpful for eventually coordinating between multiple `ayb` nodes, a single-node version should be able to store its metadata in SQLite with little setup costs.
  * [ ] Authentication and permissions. Add authentication/the ability to log in, and add permissions to endpoints so that you can't just issue queries against any database.
  * [ ] Clustering. Support for multiple `ayb` nodes to serve databases and requests. Whereas a single database will not span multiple machines, parallelism/distribution will happen across users and databases.
  * [ ] Persistence beyond the node. Using projects like [LiteFS](https://github.com/superfly/litefs), stream updates to databases to persistent storage, and allow failover if an `ayb` node disappears.
  * [ ] Isolation. Since an `ayb` instance can have multiple tenants/databases, we want to use one of the many container/isolate/microVM projects to ensure that one tenant isn't able to access another tenant's data.
  * [ ] Sessions/transactions. `ayb`'s query API is a stateless request/response API, making it impossible to start a database transaction or issue multiple queries in a session. Exposing sessions in the API will allow multiple statements per session, and by extension, transactions.
  * [ ] Import/export of databases. `ayb` already uses existing well-established file formats (e.g., SQLite). There should be endpoints to import existing databases into `ayb` in those formats or export the underlying files so you're not locked in.
* Extend `ayb` to more people and software
  * [ ] Collaboration. In addition to making it easy to create and query databases, it should be easy to share databases with others. Two use cases include adding private collaborators and allowing public read-only access.
  * [ ] Forking. Allowing a user to fork their own copy of a database will enable collaborators to remix and build on each others' work.
  * [ ] Versioning. To both make it less scary to execute sensitive operations and to make it possible for scientists to reference and publish checkpoints of their work, a user should be able to snapshot and revert to a database at a point in time.
  * [ ] DuckDB. Allowing users to create a DuckDB database in addition to a SQLite database would allow you to create a data warehouse with a single command. This effort is dependent on the DuckDB project. First, the DuckDB file format is rapidly changing ahead of the project's 1.0 release. Additionally, I don't know of an equivalent streaming replication project to LiteFS for DuckDB that handles *persistence beyond the node*.
  * [ ] PostgreSQL wire protocol. While an HTTP API makes it easy to build new web apps, exposing `ayb` over the PostgreSQL wire protocol will allow existing tools and libraries to connect to and query an `ayb` database.
* Increase discoverability with a web frontend
  * [ ] Provide a web interface analogous to the command line interface. Much like GitHub/Gitea/Forgejo make git more approachable, you shouldn't have to pay a command line knowledge tax in order to create, share, and query an `ayb` database.
  * [ ] Explore people's public datasets. Beyond simplifying the command line, platforms like GitHub also make it easier to find a user's publicly shared repositories, follow along in their work, and fork a copy for your own exploration. That same experience should be possible for `ayb`-hosted databases.

## Contributing
(This section is inspired by the [LiteFS project](https://github.com/superfly/litefs#contributing), and is just one of the many things to love about that project.)

`ayb` contributions work a little different than most GitHub projects:
* If you have a small bug fix or typo fix, please PR directly to this repository.
* If you want to contribute documentation, please PR directly to this repository.
* If you would like to contribute a feature, create and discuss the feature in an issue on this GitHub repository first. Once the feature and some of its finer details are hashed out in the issue and potentially a design document, submit a pull request. I might politely decline pull requests that haven't first been discussed/designed.

This project has a roadmap and features are added and tested in a certain order. I'm adding a little friction in requiring a discussion/design document for features before submitting a pull request to ensure that I can focus my attention on well-motivated, well-sequenced, and well-understood functionality.

