# Neorg Query (Neorq)

This project has a few names. Neorq is fun, but too easy to typo, and bad for seo.

> [!WARNING]
> Massive WIP. If you're going to use this, treat any update as a breaking change until this warning
> is gone. Also, the rust side might panic. If that happens, create a bug report. I'll remove all
> the unwrap/expects later

> [!NOTE]
> This is not [Neorg macros](https://vhyrro.github.io/posts/neorg-macros/), this is a third party
> module. Neorg's macro system doesn't exist yet

https://github.com/user-attachments/assets/5063288b-f251-4948-9cd3-114b9c59c051

A database and query interface for your notes.

---

## Install

> [!NOTE]
> You will need to have the rust toolchain installed to build this plugin, and it might take two or
> even three tries as it can hit lazy's build timeout. Each successive attempt will make it further
> thanks to caching, so just keep trying, or increase the timeout (refer to lazy docs)

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
    -- neovim. Funny enough, this is the only user facing way to trigger a full index of your
    -- workspace at the moment
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

There are three tables right now:

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

**`tasks`**

| index | name      | type        | notnull | default           | pk  |
| ----- | --------- | ----------- | ------- | ----------------- | --- |
| 0     | task_id   | INTEGER     | 1       |                   | 1   |
| 1     | file_id   | INTEGER     | 1       |                   | 0   |
| 2     | text      | TEXT        | 1       |                   | 0   |
| 3     | status    | VARCHAR(32) | 1       |                   | 0   |
| 4     | due       | DATETIME    | 0       |                   | 0   |
| 5     | starts    | DATETIME    | 0       |                   | 0   |
| 6     | recurs    | DATETIME    | 0       |                   | 0   |
| 7     | priority  | VARCHAR(32) | 0       |                   | 0   |
| 8     | timestamp | DATETIME    | 0       |                   | 0   |
| 9     | parent_id | INTEGER     | 0       |                   | 0   |
| 10    | created   | DATETIME    | 0       | CURRENT_TIMESTAMP | 0   |
| 11    | updated   | DATETIME    | 0       | CURRENT_TIMESTAMP | 0   |

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

### `#tasks`

Instead of `#format` you can tag your query with `#tasks` and this will do the formatting for you.
This essentially uses the format string: `- ( ) ${text} {:${path:$}:#${text}}[]` with some added
logic to expand the modifier extensions (TODO status, priority, etc.) and handle task nesting.

```norg
- ( ) Task Name {:$/link/to/file:# Task Name}[]
-- ( ) Child Task {:$/link/to/file:# Child Task}[]
```

Your task queries **must** join the `docs` and `tasks` tables, otherwise the links don't work.

#### Please Note:

-   Due date, Timestamp, and Start date, may not look identical due to the round trip with the
    database
-   Tasks may not maintain the same order as in the document
-   detached modifiers are in a fixed order, not the order from the source
-   Due and Start dates are unsupported by the v1 TS parser, as a result, you should only use
    timestamp for now. Even though neorg query supports all three.
-   Tasks with a modifier with a malformed date are stored as if that modifier were not there
    `- ( ) hey` and `- ( |@ Jan 1 2025) hey` are stored in the same way b/c `Jan 1 2025` is not
    a valid norg date (according to spec), it should be `1 Jan 2025`
    - I plan to eventually add a diagnostic check to
    [neorg-interim-ls](https://github.com/benlubas/neorg-interim-ls) for this, but it couldn't
    work for due dates or start dates b/c of the TS parser (just another reason to use timestamps)

## Example Queries

Please refer to the wiki, I'm going to leave it open, hopefully we can build up a nice cookbook of
useful and interesting queries.

Currently writing the queries is the hard part about using this plugin, but once they're in place,
you don't have to think about them again.

## Developers

Please checkout [the roadmap](./ROADMAP.norg) and [CONTRIBUTING.md](./CONTRIBUTING.md). And/or ask
about the project in the Neorg discord.
