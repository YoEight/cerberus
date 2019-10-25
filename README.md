Cerberus
========

An EventStore administration tool.

[EventStore] is an open-source functional database.

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
- [x] Create projection.
- [ ] Provide a packaged version to Linux (APT for inst., .deb,…etc), OSX (Homebrew) and whatever is used on Windows.
- [ ] Display cluster information.
- [ ] Expose node statistics.
- [ ] Comprehensive data migration.
- [ ] Comprehensive data export (as JSON file(s) or other human-readable format).
- [ ] Create a backup archive.
- [ ] Featureful TUI interface.

Usage
=====

The following usage example don't showcase all parameters variation. You need to run
`cerberus [COMMAND] --help` to get all parameters supported by a command.

Some commands might require a database user with the right permission. In such cases, you only need to provide `--login` and `--password` parameters.

### Check database connection

```
$ cerberus check
```

Cerberus will automatically check if a node belongs to cluster. If the node does belong to a
cluster, Cerberus will also check if node composing the cluster are reachable.

Cerberus will also return each node version.

### Create a projection

```
$ cerberus create-projection -k onetime --name amazing-proj --enabled /path/to/projection.js 
```

The name parameter is optional. In this case, the projection named `amazing-proj` will
start right away because of the flag `--enabled`.

[EventStore]: http://eventstore.org
