# MemoryCore Castle - User Guide

MemoryCore Castle is a full-screen desktop for your MemoryCore companion. It opens your own MemoryCore folder, runs **Claude Code** or **Codex** inside it, and gives your companion a home on your screen.

> This guide grows with Castle. Screenshots are added with the first release.

## Contents

1. [Getting started](#1-getting-started)
2. [Chat and Log](#2-chat-and-log)
3. [Quick actions](#3-quick-actions)

---

## 1. Getting started

### What you need

- Windows 10 or 11
- **Claude Code** or **Codex**, installed and signed in with your own account. Castle has no account of its own; your companion uses the subscription you already have.
- A MemoryCore folder. If you do not have one yet, Castle can create it for you the first time it opens.

### Opening and hiding Castle

- Castle fills your desktop and keeps the Windows taskbar visible.
- **Hide** it with the **–** button at the top right, or press **Ctrl + Alt + M**. Press the keys again to bring it back.
- Castle lives in the system tray (bottom right of the taskbar). Click its crystal icon to show or hide it. Right-click it for **Open Castle**, **Wake companion** and **Quit**.
- Closing the window only hides Castle. Use **Quit** in the tray to end it; if your companion's session is running, Castle asks first.

### Waking your companion

The panel on the left belongs to your companion. When the session is not running you see **Wake** with your companion's name. Press it to start Claude Code or Codex in your MemoryCore folder.

If Castle cannot start the session, the panel says why in one sentence and offers the fix:

| What the panel says | What to do |
|---------------------|------------|
| No MemoryCore folder is chosen yet | Press **Open setup** and choose or create your folder |
| Your MemoryCore folder was not found | Press **Settings** and choose the folder again (it may have moved) |
| Claude Code (or Codex) was not found | Install it, sign in, then press **Wake** again |

---

## 2. Chat and Log

The companion panel has two views. Switch between them with **CHAT** and **LOG** at the top of the panel; the session keeps running either way.

### Chat

Chat shows your conversation as message bubbles, like a messaging app.

- Type in the box at the bottom and press **Enter** to send. Press **Shift + Enter** for a new line.
- While your companion is working (reading files, running tools), Chat shows a typing bubble. The detailed steps stay in the Log.
- Pictures your companion makes appear in Chat; click one to see it large. Voice lines play one at a time, and you can play them again.
- **Share a picture:** drag it onto Chat, or paste it with **Ctrl + V**. It shows above the box before you send.
- A plan your companion proposes appears as a **PLAN** card; click it to read the whole plan.
- Ticks on your messages: one tick means it is waiting in the queue (your companion reads it after its current step); two ticks mean it was read.

**Claude Code:** when your companion asks you to choose (for example, to allow a command), the choices appear as buttons in Chat.

**Codex:** when Codex asks for your approval, Chat shows *"is asking for your approval"* with an **Answer in Log** button. Answer it in the Log.

### Log

The Log is the real terminal underneath, exactly as Claude Code or Codex shows it. Use it for anything Chat does not show.

- **Copy:** select text and press **Ctrl + C**, or right-click.
- **Paste:** **Ctrl + V**, or right-click with nothing selected. A copied picture is saved and its file path is pasted.
- Drop a file on the Log to type its path.

### Making the panel bigger

- Drag the panel's right edge to make it wider or narrower.
- The **⤢** button gives the panel the whole screen; **⤡** puts it back beside the widgets.

---

## 3. Quick actions

You do not need to remember MemoryCore's commands. The row of buttons under **CHAT | LOG** types them for you.

| Button | When it shows | What it does |
|--------|---------------|--------------|
| **Save diary** | Always | Asks your companion to write today into your daily diary |
| **Save project** | When your MemoryCore has projects | Saves the project you are working on |
| **Save memory** | When the save-memory skill is installed | Saves what your companion learned this session |

- The buttons are greyed out until your companion's session is running.
- After a click the button shows **Sent ✓** for a moment, and your companion answers in Chat as usual.
- You can add your own buttons (a label and the words to type) in **Settings**.
