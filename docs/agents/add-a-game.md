# open-recomp: add a game (instructions for coding agents)

The person wants a game listed on the open-recomp dashboard
(https://leonardochappuis.github.io/open-recomp/). New entries are reviewed by
a maintainer before they appear; until then they show under "Unfiltered".

## 1. Collect the details (ask the person; don't guess)

| Field | Form id | Required | Notes |
|---|---|---|---|
| Original game name | `game_name` | yes | e.g. "Sky Garden" |
| Recomp project name | `recomp_name` | no | e.g. "Sky Garden HD" |
| Console | `console` | yes | one of the options in the form (n64, gc, wii, switch, ps1, ps2, ps3, psp, xbox, x360, ...) |
| Title id or serial | `title_id` | yes | e.g. `4D5307E6`, `SLUS-20062`, `NSGE` |
| Status | `status` | yes | planned, in-progress, boots, playable, complete |
| Runs on | `targets` | no | Windows, Linux, macOS, Android, Browser |
| Recomp repository | `repo` | no | GitHub `owner/name` |
| Cover image | `cover` | no | an https image URL; or the person drags the image into the form |
| Year | `year` | no | |
| Description | `description` | no | one or two sentences |

## 2. Submit it (pick one)

**A. Prefilled form (works for anyone, nothing to install).** Build this URL,
URL-encoding every value, and give it to the person to open and submit:

```
https://github.com/leonardochappuis/open-recomp/issues/new?template=add-game.yml&title=%5Bgame%5D+<recomp or game name>&game_name=<...>&recomp_name=<...>&title_id=<...>&repo=<...>&cover=<...>&year=<...>&description=<...>
```

Dropdowns (console, status) and checkboxes (targets) can't be prefilled; tell
the person to pick them in the form. They can also drag a cover image into the
cover box before submitting.

**B. With orc (the person has orc and gh set up):**

```bash
orc game add --platform <console> --id <title id> --name "<game>" --title "<recomp name>" \
  --status <status> --target windows --target linux --repo <owner/name> --cover-file <image>
```

Run it inside any project that has `.orc/project.toml` pointing at the hub,
or a clone of the hub itself. It opens a pull request.

## 3. Rules

- Never upload ROMs, ISOs, extracted game files or copyrighted assets other
  than box art the person has the right to share.
- Don't submit the same game twice: search the dashboard first.
