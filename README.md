# Neorg Query (Neorq)

This project has a few names. Neorq is fun, but too easy to typo, and bad for seo.

> [!WARNING]
> Massive WIP. If you're going to use this, treat any update as a breaking change until this warning
> is gone.

> [!NOTE]
> This is not [Neorg macros](https://vhyrro.github.io/posts/neorg-macros/), this is a third party
> module. Neorg's macro system doesn't exist yet

https://github.com/user-attachments/assets/5063288b-f251-4948-9cd3-114b9c59c051

A database and query interface for your notes.

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

You can use this plugin as a category completion source for
[neorg-interim-ls](https://github.com/benlubas/neorg-interim-ls). See that readme--there is no
additional configuration needed in this plugin.

### `#sql`

You can tag a sql statement that's in a free form verbatim with `#sql` and `#format
<format-string>`, and neorq will evaluate the query (against a read only database connection),
format the results according to the format string, and inline the results. The query can span
multiple lines, and doesn't need a trailing `;`

Here's an example before running:

```norg
#sql
#format `|- {https://github.com/benlubas/${path:t}}[${title|path:t}] - ${description}|`
`|SELECT path, title, description FROM docs d
JOIN categories c ON c.file_id = d.id
WHERE c.name = 'neorg'
ORDER BY created|`
```

And after running it might look like this:

```norg
#sql
#format `|- {https://github.com/benlubas/${path:t}}[${title|path:t}] - ${description}|`
`|SELECT path, title, description FROM docs d
JOIN categories c ON c.file_id = d.id
WHERE c.name = 'neorg'
ORDER BY created|`
___
- {https://github.com/benlubas/neorg-conceal-wrap}[Neorg Conceal Wrap] - Hard wrap based on concealed width
- {https://github.com/benlubas/neorg-interim-ls}[Neorg Interim LS] - A Language Server in Lua for Neorg
- {https://github.com/benlubas/neorg-module-tutorial}[Neorg Module Tutorial] - Walking through Neorg's module system with a tangle-able norg file
- {https://github.com/benlubas/neorg-query}[Neorg Query] - Database integration for neorg
___
```

**Tag order doesn't matter**.

You can also do this:

```norg
#sql `|select * from docs where path like '%/test/%';|`
#format `|- {:${path:$}:}[${title|description|path:t}] - ${indexed}|`
```

and this:

```norg
#format `|- {:${path:$}:}[${title|description|path:t}] - ${indexed}|`
#sql `|select * from docs where path like '%/test/%';|`
___
___
```

But **not** this:

```norg
#sql `|select * from docs where path like '%/test/%';|`
#format `|- {:${path:$}:}[${title|description|path:t}] - ${indexed}|`
___
```

#### Tables

There are two tables right now:

**`docs`**: contains information about documents and their metadata

| index | name        | type          | notnull | default           | pk  |
| ----- | ----------- | ------------- | ------- | ----------------- | --- |
| 0     | id          | INTEGER       | 0       |                   | 1   |
| 1     | path        | VARCHAR(1024) | 1       |                   | 0   |
| 2     | title       | TEXT          | 0       |                   | 0   |
| 3     | description | TEXT          | 0       |                   | 0   |
| 4     | authors     | TEXT          | 0       |                   | 0   |
| 5     | created     | DATETIME      | 0       |                   | 0   |
| 6     | updated     | DATETIME      | 0       |                   | 0   |
| 7     | indexed     | DATETIME      | 0       | CURRENT_TIMESTAMP | 0   |

`indexed` is used internally to determine if a file needs to be re-indexed when you open neorg.

**`categories`**

| index | name    | type         | notnull | default | pk  |
| ----- | ------- | ------------ | ------- | ------- | --- |
| 0     | id      | INTEGER      | 0       |         | 1   |
| 1     | file_id | INTEGER      | 0       |         | 0   |
| 2     | name    | VARCHAR(255) | 1       |         | 0   |

### `#format`

Very basic format. Include the value of a column with `${column_name}`. If you select a col with `AS
something` you will use `something`, otherwise, use the regular column name. If you want to include
a literal `${name}` tough luck. There's no way to do that right now. If you have a good reason to
want this, open an issue, we can figure something out.

#### Fallback

If a field is null, you can fall back to a different field with `|`. eg: `${title|description}`, If
the row has title, this will evaluate to the title. If it doesn't, it will evaluate to the
description.

If all provided values are null, an empty string is used

#### Modifiers

You can add modifiers to fields with `:mod`. eg: `${path:$}` applies the `$` mod to `path`. With
fallback this looks like: `${path:$|else}` or `${title|path:$}`, in each case, the `$` is only
applied to path. Multiple modifiers would look like this: `${path:$:mod}`. Mods will be applied in
order. Not all mods are compatible.

**Path Modifiers:**

-   `$` convert a path to its workspace relative representation. Must receive a path. eg:
    `/home/me/notes/note.norg` -> `$/note`
-   `t` "Tail" of a path without extension. Same as the `:h filename-modifier` `:t`. eg:
    `/home/me/notes/note.norg` -> `note`

**Other Modifiers:**

They don't exist yet. If you have ideas, open an issue/PR

## Developers

Please checkout [the roadmap](./ROADMAP.norg) and [CONTRIBUTING.md](./CONTRIBUTING.md). And/or ask
about the project in the Neorg discord.
