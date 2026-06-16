<div align="center">

# 🧑‍💻 Jbuddy

**Your job buddy — a friendly menu-bar companion that helps you work healthier, not longer.**

Jbuddy lives in your macOS menu bar, measures how much you *actually* worked, and gently
reminds you to drink water, stretch, and rest your eyes — with a little mascot whose mood
reflects your habits. 100% local. Zero telemetry.

</div>

---

> ⚠️ **Status: under active rewrite.** Jbuddy used to be a "stay-awake" tool that jiggled the
> mouse to keep chat apps "Available". That's gone. It's being rebuilt from scratch into a
> genuine well-being companion. See [Roadmap](#roadmap).

## Why

Most "productivity" tools push you to do *more*. Jbuddy does the opposite: it helps you keep a
**sustainable** rhythm. It tracks your **real** focus time (from actual keyboard/mouse activity —
never by faking it), nudges you to take care of yourself, and warns you when you're overdoing it.

## Features

- ⏱️ **Real work tracking** — measures effective focus time from genuine activity, splitting your
  day into *working · idle · away*. When you step away, the clock stops. The numbers are honest.
- 💧 **Healthy reminders** — water, stand-up & stretch, and the 20-20-20 eye rule. Every interval
  is configurable, and reminders won't interrupt you mid-meeting.
- 🛑 **Anti-overwork guard** — nudges you when you've passed your target hours or gone too long
  without a break.
- 📊 **Honest reports** — end-of-day and weekly view of effective hours, plus a heatmap of your
  most productive and most idle hours.
- 🐣 **A buddy that cares** — a mascot in the tray whose mood mirrors your habits. Stay hydrated
  and take breaks → it thrives. Grind nonstop → it gets tired. Light gamification, no pressure.
- 🔒 **Private by design** — everything stays on your machine. No accounts, no cloud, no tracking.

## Tech

Built with **[Tauri](https://tauri.app/)** (Rust core + web frontend) — a tiny, fast, native
menu-bar app. macOS first; Windows support planned.

```bash
pnpm install
pnpm tauri dev      # run in development
pnpm tauri build    # build the .app
```

Requirements: Node, pnpm, and the Rust toolchain.

## Roadmap

- [x] Project rewrite & Tauri scaffold
- [x] Real idle/activity tracking (working · idle · away)
- [ ] Configurable reminders (water · stretch · 20-20-20)
- [ ] Dashboard: today, week, and hourly heatmap
- [ ] Anti-overwork guard & end-of-day nudge
- [ ] Mascot moods & light gamification
- [ ] Launch at login + polished release

## License

See [LICENSE](LICENSE).
