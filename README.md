# neorg-query

This is a toy/POC concept for a plugin that allows dynamic query running to populate sections of
your notes by running a SQL query.

(Planned) Features:
- [ ] Persistent LibSQL database that tracks file metadata and TODO items in each file
- [ ] Parse files with rust-norg parser
- [ ] `@query` tag that populates on `:Neorg query run`
- [ ] Update the database on launch
    - [ ] store file modified dates probably
    - [ ] remove files that are deleted
    - [ ] add/parse files that are new
- [ ] Watch current workspace and update on file changes. Debounce this.
