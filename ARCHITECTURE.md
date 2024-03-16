# Overview

Buckets is a command line interface tool which uses command lin eparsing to execute specific actions
based on the users input.

# Architecture

## Entry point

Buckets is a command line tool where actions are given on the command line. E.g. `bucket init fungame` initialises
a Bucket repository called 'fungame'. In `main.rs` there is a function called `cli()` which is the entry point for
the command line interface. This function uses the `clap` crate to parse the command line arguments and execute the
appropriate action. The parsed arguments is then matched against the possible commands and the appropriate function
is called in the command module.

## Commands

The commands are defined in the `commands` module. Each command is a function which takes the parsed arguments as input
and returns a `Result` type. The `Result` type is used to handle errors. If the command is successful, the `Ok` variant
is returned, otherwise the `Err` variant is returned with an error message.

## Storage

### Metadata

Metadata is stored in a SQLite database. The database is created in the `.bucket` directory in the root of the and is
called `bucket.db`. The database has a table called `meta` which stores the metadata of the repository.

# Repository directory layout

## **Top level**

`.buckets` Contains general information.In a monorepo this is the top level directory

`.buckets\config`Bucket repository configuration file.

`.buckets\bucket.db` Repository metadata database

## **Per bucket container**

`.b` At the top of a bucket, contains general information:

`.b\info.toml` Bucket configuration file

### **Commits**

`.b\storage` Top level of all stored content.
