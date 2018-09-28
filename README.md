# Lightweight embedded database

[![License: MIT](https://img.shields.io/badge/License-MIT-brightgreen.svg)](https://opensource.org/licenses/MIT)
[![Travis-CI Build Status](https://travis-ci.org/katyo/ledb.svg?branch=master)](https://travis-ci.org/katyo/ledb)
[![Appveyor Build status](https://ci.appveyor.com/api/projects/status/1wrmhivii22emfxg)](https://ci.appveyor.com/project/katyo/ledb)
[![Crates.io Package](https://img.shields.io/crates/v/ledb.svg?style=popout)](https://crates.io/crates/ledb)
[![Docs.rs API Documentation](https://docs.rs/ledb/badge.svg)](https://docs.rs/crate/ledb)

The **LEDB** is an attempt to implement simple but efficient, lightweight but powerful document storage.

The abbreviation *LEDB* may be treated as an Lightweight Embedded DB, also Low End DB, also Literium Engine DB, also LitE DB, and so on.

## Documents storage library (`ledb` crate)

This is a basic library which implements document storage and query functionality.

See the [README](ledb/README.md).

## Actor and REST-interface for documents storage (`ledb-actix` crate)

This is an actor which helps interacting with database in applications which builts on the [actix](https://actix.rs/) actor framework.

See the [README](ledb-actix/README.md).
