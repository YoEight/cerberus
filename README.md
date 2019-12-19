Cerberus
========

![Crates.io](https://img.shields.io/crates/v/cerberus-es.svg)
![Crates.io](https://img.shields.io/crates/d/cerberus-es.svg)
![Discord](https://img.shields.io/discord/415421715385155584.svg)
![Crates.io](https://img.shields.io/crates/l/cerberus-es.svg)

An EventStore administration tool.

[EventStore] is an open-source functional database.

[Talk and exchange ideas in our dedicated Discord Server]

Goal
====

Cerberus aims at providing an easy access to EventStore database in the comfort of our beloved terminal.

Feature list
============

Those features are not listed into an implementation order.

- [x] Show a database node/cluster version.
- [x] Database healthiness check-up (cluster mode included).
- [x] Comprehensive events listing.
- [x] Comprehensive stream listing.
- [x] Comprehensive projection listing.
- [x] Comprehensive persistent subscription lising.
- [x] Create persistent subscription.
- [x] Update persistent subscription.
- [x] Delete persistent subscription.
- [x] Create projection.
- [ ] Provide a packaged version to Linux (APT for inst., .deb,…etc), OSX (Homebrew) and whatever is used on Windows.
- [ ] Display cluster information.
- [ ] Expose node statistics.
- [x] Comprehensive data migration.
- [ ] Comprehensive data export (as JSON file(s) or other human-readable format).
- [x] Create a backup archive.
- [x] Apply or check database compliance (If a database contains all the persistent subscriptions, projections, or other requirement needed)
- [ ] Featureful TUI interface.
- [ ] Install EventStore locally.

Install
=======

Currently, we don't provide compiled binaries. However, we aim at providing means to install `cerberus`
through OS package managers:

* APT for debian-based Linux distributions (a .deb file will be available too).
* Homebrew formula for OSX.
* Whatever Windows package manager.

### From source

`Cerberus` is written in Rust. To install Rust dev tools, go to: https://rustup.rs/

#### From Crates.io

```
$ cargo install cerberus-es
```

The program will be available in `~/.cargo/bin`

#### From source repository

Clone https://github.com/YoEight/cerberus.git git repository. Once you are in `cerberus` directory run:

```
$ cargo build --release
```

The binary will be in `target/release/` directory.

Common usages
=============

The following usage example don't showcase all parameters variation nor all commands. You need to run
`cerberus --help` to get all supported commands or, `cerberus [COMMAND] --help` to get all parameters supported by a command.

Some commands might require a database user with the right permission. In such cases, you only need to provide `--login` and `--password` parameters.

## Check database connection

```
$ cerberus check
```

Cerberus will automatically check if a node belongs to cluster. If the node does belong to a
cluster, Cerberus will also check if node composing the cluster are reachable.

Cerberus will also return each node version.

## List events

```
$ cerberus list-events --stream foo
```

This command lists every events of the `foo` streams.

```
$ cerberus list-events --stream foo --recent
```

This will command will do the same but will only take the last 50 `foo` 's events.

```
$ cerberus list-events --stream foo --group-id my_group
```
This command lists all parked events that belong to the persistent subscription targetting the stream `foo` and the group
`my_group`.

```
$ cerberus list-events --stream foo --group-id my_group --checkpoint
```
This command is similiar to the previous one but will list all the persistent subscription checkpoints.

```
$ cerberus list-events --by-type user-created
```

This command lists all the events with the type `user-created`

## List streams

```
$ cerberus list-streams
```

This command lists all user-defined streams plus deletion and and metadata streams.

```
$ cerberus list-streams --by-category user
```

This command lists all streams that start with `user-`.

## Create a projection

```
# You will have to use an user with admin right to use that command.
$ cerberus --login=boogie_man --password=******* create-projection -k onetime --name amazing-proj --enabled /path/to/projection.js
```

The name parameter is optional. In this case, the projection named `amazing-proj` will
start right away because of the flag `--enabled`.

## Backup

Cerberus supports database backup. That feature requires `rsync` to be in `$PATH`. Currently,
when in a cluster configuration, Cerberus doesn't backup all the nodes nor it selects the master node.
We might implement those features mentioned above based on users demand.

Usage:
```
cerberus --host=1.2.3.4 backup --remote-user=yoeight --source-directory=/var/eventstore/db --destination-directory=/home/yoeight/es-backup
```

## Compliance checking

Compliance checking makes sure a database setup is properly configured based on a compliance file. A compliance file
uses the TOML format. In a compliance file, you can make sure that:
* Persistent subscriptions are created and also have all their configuration up-to-date.
* User-defined projections are created and also have all their configuration up-to-date.
* General database configuration (version, cluster configuration,…etc) WIP.

Example of compliance file:
```toml
[[projection]]
name = "foo-bar-indexing"
path = "path/to/my/amazing/code.js" # Absolute or relative filepath.
type = "continuous"                 # [possibilities: "continuous", "one-time"]

# Optional settings
emit = true # Optional [default: false]. Enable the ability for the
            # projection to write to streams.

enabled = true # Optional [default: false]. Indicates if the projection
               # will start right away when created.

checkpoints = true # Optional [default: false]
                   # [when projection's type is "continuous", forced at true]
                   # Enable persisting projection progress.

track_emitted_streams = true # Optional [default: false]
                             # Write the name of the streams the projection is
                             # managing to a separate stream:
                             # $projections-{projection-name}-emittedstreams

[[subscription]] # Think persistent subscription.
stream = "the_world" # The stream doesn't need to exist when creating the subscription.
group = "illuminatis"
start_from = 0 # Event number.

# Optional settings
resolve_link           = false         # Optional [default: false]
extra_stats            = false         # Optional [default: false]
msg_timeout_in_ms      = 10_000        # Optional [default: 10_000]
max_retry_count        = 10            # Optional [default: 10]
live_buffer_size       = 500           # Optional [default: 500]
buffer_size            = 500           # Optional [default: 500]
read_batch_size        = 20            # Optional [default: 20]
checkpoint_after_in_ms = 1_000         # Optional [default: 1_000]
min_checkpoint_count   = 10            # Optional [default: 10]
max_checkpoint_count   = 500           # Optional [default: 500]
max_subscriber_count   = 10            # Optional [default: 10]
strategy               = "round-robin" # Optional [default: "round-robin"] [possibilities: "round-robin", "dispatch-to-single", "pinned"]
```

Usage:
```
# You will have to use an user with admin right to use that command.
cerberus --login=boogie_man --password=******* apply-compliance --file=compliance.toml
```

Note: if you only want to see if your database configuration is up-to-date without updating
the database itself, use the `--dry-run` flag. You'll got a complete report without any change
pushed to the database.

Notes
=====

That binary was tested on Linux and OSX.
Contributions and bug reports are welcome!

MIT License.

[EventStore]: http://eventstore.org
[Talk and exchange ideas in our dedicated Discord Server]: https://discord.gg/x7q37jJ
