# 0017. Local usage statistics with milestones

- Status: Accepted
- Date: 2026-10-23

## Context

People keep using a tool when they see what it gives them. Typlet saves typing, but that saving is invisible: a trigger turns into text and the moment is gone. A small dose of gamification (how much time was saved, a streak, milestones) makes the benefit visible and encourages people to create more snippets.

A text expander sees every key press, so anything it records is sensitive. Statistics must not become a keylogger by accident, must not leave the computer, and must not interrupt typing.

## Decision

- The background program counts each successful insertion in `stats.json` in the data folder ([`stats.rs`](../../crates/typlet-core/src/stats.rs), [paths](../../crates/typlet-platform/src/paths.rs)). The file holds **numbers, dates and snippet ids only**: insertions in total and per day, characters saved, the best streak, and per snippet the count and the last day it was used. Never text, triggers, app names or times of day.
- **Characters saved** are the inserted characters minus what the user typed for them: the trigger and any form answers. **Time saved** is estimated at 200 characters per minute, an average typing speed. An expansion undone with Backspace is taken back.
- Daily counts are kept for about a year (for streaks and the activity chart); totals forever. A **streak** is the number of days in a row with at least one insertion; it stays alive until the end of the next day.
- **Milestones** (10 to 10 000 insertions, 1 to 100 hours saved, streaks of 3 to 365 days) follow from the totals, so nothing extra is stored. A new milestone appears **quietly as a tray menu entry**, never as a window: a window could take the focus and catch what the user is typing.
- The background program reads the file before every change and writes it atomically. The editor shows the numbers (statistics page, a line on the start page) and can **reset** them by deleting the file. A damaged file is set aside as `stats.json.damaged` and counting starts over.
- Counting can be switched off in the settings (`statistics`). Nothing is ever transmitted; diagnostics reports do not include the statistics.

## Consequences

- Users see what Typlet does for them, and which snippets they actually use.
- The privacy promise stays simple: Typlet stores counts, never what you type. The user guide and README say so.
- Reading and writing a small file per insertion costs a few milliseconds on the user-interface thread, after the text is already inserted.
- The time estimate is deliberately simple and conservative; it is labelled as an estimate.
