# Narguis' Notes App

My personal notes and planner app, built around my organization needs and personal usage. It includes a series of features I find useful when keeping up with my day to day schedule.

The app is meant to work locally with no connection and is under development / UX improvements.

## Features

### Planner

The main view, here you can navigate through days (each being one page) and manage your tasks for that specific day.

On each line:

- Click the `+` button to expand / collapse the description field to add more details on a task.
- Click the clock symbol to set a time for that line / task.
- Click the bell symbol to toggle alarms for that task on / off. Alarms play 15 minutes before and on due time for each task.
- Click the checkbox to cross the line / mark it as done.
- Click the `...` menu to move a task to tomorrow, save it as a recurring task, or delete the line.

Navigation:

- Use the arrow keys and `Go to Today` buttons to move between planner pages.
- Switch to the weekly plan for a separate free-text weekly view and navigate week to week.
- Use the `Unfinished` tab to see what tasks were left open and go to them.
- Use the `Import from tasks` button to add a recurring task to that day's schedule.

### Notes

Notes are a tab for writing up ideas and quick reminders that aren't specific to a particular day.

- The Notes view opens to a grid of existing notes.
- Choose `Create new` to write a note.
- Click an existing note to edit or delete it.

### Tasks

You can create recurring task definitions for things you do repeatedly.

- The Tasks tab opens to a grid of saved tasks.
- Choose `Create task` to define a new recurring task.
- A recurring task can have a title and optionally, set details and time.
- Optionally, you can set a deadline or define which weekdays the task repeats over.
- The current weekday's tasks appear separate from the rest on the Tasks tab.

### Timers

Here, you can set countdowns as timers / reminders.

- Click `New timer` to create a new countdown with a Title and time in hours and minutes.
- The Timers tab shows all running timers with remaining time and the expected finish time.
- Upon finish, timers trigger an alarm that uses the same sound and popup alert behavior as task alarms.
- Alarms and timers require the app to be running; desktop notifications may also require operating-system permission.

## Install and Launch

The app is distributed as a Debian package for Ubuntu and other Debian-based Linux systems.

To build, install, update, and launch the app from this repository:

```bash
bash scripts/install-update.sh
```

Run the same command again after making changes. It rebuilds the package and upgrades the installed app in place; you do not need to uninstall first.

After installation, you can open `Narguis Notes` from the desktop application menu. The installed app runs independently and does not need Node, pnpm, Rust, or a development server.

## Data and Backups

The installed app stores its main database in the user's application-data directory:

```text
$XDG_DATA_HOME/com.narguis.notes.desktop/notes-planner.sqlite3
```

If `XDG_DATA_HOME` is not set, it uses:

```text
~/.local/share/com.narguis.notes.desktop/notes-planner.sqlite3
```

Your data stays on the computer. Daily lines, notes, tasks, and the native database are local to the current user profile. The weekly free-text plan is stored locally per week. When a database operation is temporarily unavailable, unsynced changes are kept in the installed app's local storage and retried when database access returns. This fallback is persistent for the same user profile, but it is not encrypted.

For a backup, stop the app and use SQLite's online backup command:

```bash
sqlite3 "$XDG_DATA_HOME/com.narguis.notes.desktop/notes-planner.sqlite3" \
  ".backup '/secure-backups/notes-planner.sqlite3'"
```

When `XDG_DATA_HOME` is unset, use `~/.local/share` in the path.

## Technology

The app uses Tauri 2 for the desktop window, TypeScript and Vite for the interface, Rust for native commands, and SQLite for local persistence. It has no account system, cloud sync, telemetry, or required network service at this point.

## License

MIT. See `LICENSE`.
