# Neorg Query

> [!WARNING]
> Massive WIP. If you're going to use this, treat any update as a breaking change until this warning
> is gone.

---

## Install

> [!NOTE]
> You will need to have the rust toolchain installed to build this plugin

<details>
  <summary>Lazy.nvim</summary>

Add this to `nvim-neorg/neorg`'s dependencies.  
**Make sure that you have luarocks support enabled.**
```lua
{ "benlubas/neorg-query" }
```
</details>

<details>
  <summary>Rocks.nvim</summary>

`:Rocks install neorg-query`
</details>

## Config

Load the module by adding it to your neorg configuration.

```lua
-- default values
["external.query"] = {
    -- Populate the database. Indexing happens on a separate thread, so doesn't block
    -- neovim. We also
    index_on_launch = true,

    -- Update the db entry when a file is written
    update_on_change = true,
}
```

## Usage

The only way to use this plugin at the moment is as a category completion source for
[neorg-interim-ls](https://github.com/benlubas/neorg-interim-ls). There is no additional
configuration needed in this plugin.

## Developers

Please checkout [the roadmap](./ROADMAP.norg) and [CONTRIBUTING.md](./)
