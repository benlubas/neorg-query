# Contributing

That's for considering contributing! This plugin is in 3 parts: rust, the lua api, and the neorg
module. I'll go over each in order, but I'll start with how they all work together.

The rust portion of this application is where all of the document parsing and database interfacing
happens. It will also be responsible for parsing the eventual basic query language, and formatting
those results.

All of the rust functions are asynchronous, so they need to be wrapped on the lua side, this is what
the lua API does. And finally, the neorg module is responsible for all of the user facing features
of the plugin, and it requires the lua API so that it can call out to rust. All of the API functions
are callback style functions.

## Rust

[`./src/lib.rs`](./src/lib.rs)

There are quite a few quirks here. I'll try to explain them all. It's probably worth pulling the
code up along side so you know what I'm talking about.

Rust is using the [`mlua`](https://github.com/mlua-rs/mlua) crate in "module mode" to create a lua
module out of rust functions. Each function is async, and this has some interesting implications for
the rest of our program.

We have to create a tokio runtime ourselves. This is stored as a lazy static.
The tokio runtime is not running in each function, so we have to start it ourselves.

We could start the runtime by acquiring a guard with `let _guard = TOKIO.enter();`. However, these
guards **must** be dropped in the reverse order they're acquired in, and you might imagine that this
is nearly impossible for us, as we want to be able to call multiple functions successively and have
them running at the same time. The necessarily acquires more than one guard, and we have no control
over which functions finish when.

So instead, we use `let handle = TOKIO.handle();` and `handle.spawn().await` to do our async work.
Note that you can still do work outside of the spawned task, but it will block. You can return
a value from the spawned task and in turn return that from the rust function. This value will be
available through the lua API as the argument to a callback function.

## Lua API

[`./lua/neorg_query/api.lua`](./lua/neorg_query/api.lua)

This is a very basic, but very important part of the application. Each function that we create and
export from the rust module must be wrapped for use. The `wrap` function takes a rust function,
and returns a function which when called, wraps the rust function in a coroutine and starts to poll
it. Under the hood, `mlua` has some code which is polling the rust function forward each time we
poll the coroutine. Importantly, tokio tasks start running immediately, and move forward without
polling. So it doesn't really matter how poorly we poll the coroutine, we're just checking on it,
and eventually it will be done. We could sleep for 10 seconds and poll it once, and it would be
finished.

## Neorg Module

WIP, need to push debugging fix
