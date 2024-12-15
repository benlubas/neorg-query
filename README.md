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

## SQL

You can create a `|neorq` ranged tag, provide a SQL query, tag it with `#sql` and provide a format
string for the results. Then running `:Neorg query run` will run the query, format the results, and
update the contents of the ranged tag with those results!

```norg
#sql
#format `|- {:${path:$}:}[${title|path:$}] - ${updated}|`
|neorq
`|SELECT * FROM docs WHERE path LIKE '%/test/%' ORDER BY updated;|`
results will end up here!
|end
```

## Format strings

Very basic format. Include the value of a column with `${column_name}`. If you select a col with `AS
something` you will use `something`, otherwise, use the regular column name.

### Fallback

If a field is null, you can fall back to a different field with `|`. eg: `${title|description}`. If
the row has title, this will evaluate to the title. If it doesn't, it will evaluate to the
description.

### Modifiers

You can add modifiers to fields with `:mod`. eg: `${path:$}` applies the `$` mod to `path`. With
fallback this looks like: `${path:$|else}` or `${title|path:$}`, in each case, the `$` is only
applied to path. Multiple modifiers would look like this: `${path:$:mod}`. Mods will be applied in
order. Not all mods will be compatible.

**Modifier List:**

-   `$` convert a path to its workspace relative representation. Must receive a path. eg:
    `/home/me/notes/note.norg` -> `$/note`

## Developers

Please checkout [the roadmap](./ROADMAP.norg) and [CONTRIBUTING.md](./CONTRIBUTING.md)
