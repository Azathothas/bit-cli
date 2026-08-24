# NetDrop - User Guide

*Files fly directly between devices. No servers, no accounts, no "upload to the cloud and share a link."*

> "The internet was meant to be a network of equals, not ten servers that everyone else connects to."
> - inspired by Tim Berners-Lee

---

## 1. What Is This App

**NetDrop** is a program for instant file and folder transfer **directly between devices**: from a computer to a phone, from a phone to a computer, from a friend's laptop to your PC - anywhere in the world.

Imagine you run an invisible cable between two devices. Everything you send travels along that cable and **never** touches a third-party server. No one along the way can read your file, save it "just in case," or serve you a drill ad because you forwarded a photo of a drill.

Runs on **Windows, Linux, macOS, and Android**. Under the hood: Rust, the QUIC protocol, and the iroh P2P network - the same technologies powering the modern "direct" internet.

**What the transfer looks like from the user's perspective:**
1. The sender picks a file > a short **ticket** and a QR code appear.
2. The receiver scans the QR (or pastes the ticket) > sees what's being offered and taps "Accept."
3. The file flies straight through. Done.

For your own devices you can skip steps 1–2 entirely - see "Trusted Devices."

---

## 2. What Problems It Solves

| The usual pain | How NetDrop fixes it |
|---|---|
| "Send me the photos" > half an hour fiddling with a cable that's always the wrong one | QR code > files are already on your computer |
| A messenger crushes video into a 480p mess | Files transfer **byte-for-byte**, no re-encoding |
| "File is over 2 GB, upload it to the cloud" | No size limit - go ahead and send a 200 GB disk image |
| Cloud = your files on someone else's computer | Files **never leave** your devices |
| Connection drops at 90% - start all over again | **Resume**: the transfer picks up where it left off |
| AirDrop doesn't work with Windows and Android | NetDrop plays nicely with everyone |
| "Email it to yourself" (the classic!) | Your phone hands the file off to your home PC - even from the street |

And the biggest, invisible problem: **privacy**. Every file that passes through a central server is a file that can be read, indexed, and stored. NetDrop solves this at the root: there is no server.

> "The argument 'I have nothing to hide' is like saying 'I have nothing to say' while giving up free speech."
> - Edward Snowden

---

## 3. How NetDrop Differs from the Rest

Centralized services are like mail with a sorting center: your package first goes to the company's warehouse, gets weighed, scanned, stored, and then (maybe) delivered to the recipient. NetDrop is handing the package face-to-face - and when arms are too short, it goes through an encrypted pipe that even the pipe-laying party cannot peek into.

| | **NetDrop** | AirDrop | Telegram / WhatsApp | Google Drive / Clouds | USB Flash Drive |
|---|---|---|---|---|---|
| Direct P2P transfer | ✅ | ✅ (nearby only) | ❌ | ❌ | ✅ (by hand) |
| Works over the internet (different networks) | ✅ | ❌ | ✅ | ✅ | ❌ |
| Windows + Linux + macOS + Android | ✅ | ❌ Apple only | ✅ | ✅ | ✅ |
| File stored on a third-party server | ❌ never | ❌ | ✅ | ✅ | ❌ |
| End-to-end encryption | ✅ X25519 + ChaCha20 | ✅ | partial | ❌ | 🤷 |
| Size limit | none | none | 2–4 GB | paid plans | flash drive size |
| Photo / video re-compression | no | no | **yes** | no | no |
| Resume after interruption | ✅ | ❌ | partial | ✅ | - |
| Account / registration required | **no** | Apple ID | phone number | Google account | no |
| On-the-fly compression | ✅ ZSTD | ❌ | ❌ | ❌ | ❌ |
| Open source | ✅ | ❌ | partial | ❌ | 😄 |

**Key difference even from other P2P drops** (Snapdrop, LocalSend): they only work within a single local network. Thanks to the iroh network, NetDrop connects devices **across any networks and NATs** - it punches through a direct channel (hole-punching, IPv4 and IPv6), and if the ISP is really restrictive, it transparently routes through an encrypted relay that sees only a stream of random bytes.

---

## 4. All Features and Use Cases

### 🎫 Tickets - Transfer to Anyone
Pick files > instantly receive a ticket `nd1...` and a QR code. A ticket is an "address + key" in a single string: forward it any way you like (show the QR, dictate it, drop it in a chat). A ticket is single-use - once the transfer is done it's as useless as a used movie stub.

*Example: your neighbor asks for an 8 GB wedding video. Show them the QR on your screen - within a few minutes the video is on their device, in the original quality.*

### 👥 Multi-Recipient
Enable it in settings and a single ticket can be used by several people at once; each downloads in parallel with their own progress bar. The ticket stays alive until you cancel it.

*Example: hand out lecture materials to the whole study group - one QR on the projector.*

### 🤝 Trusted Devices - Magic for Your Own Gear
Pair your phone and computer once (QR from settings), enable "Always Accept" on the PC, and from then on:

*You're walking around town, snap something important > "Share" > NetDrop > "⇢ Desktop-Home" - your home PC already accepted the photo and saved it to disk. You haven't even crossed the street yet.*

- Works **from any network**: mobile data, a café, another country.
- Symmetric: both the PC and the phone can listen.
- The device list shows who is currently **online** (green dot).
- A persistent device ID is used only for your own devices; one-off tickets use disposable keys (you can't be tracked).

### 📱 Android as a First-Class Citizen
- "Share" from the gallery and any app straight into NetDrop.
- Camera QR scanner - for tickets and pairing codes.
- Received files land in **Download/netdrop** - visible in the file manager and gallery.
- "Share" button on a received file - forward it immediately.
- Compact interface with bottom navigation.

### 🔐 Security Without the "I Agree to Everything" Button
- End-to-end encryption: X25519 (key exchange) + ChaCha20-Poly1305 (stream).
- **Perfect Forward Secrecy**: keys are single-use - even if someone steals your device tomorrow, yesterday's transfers can't be decrypted.
- A session fingerprint is shown to both sides - you can verify it out loud, like a password from a spy movie.
- The receiver always sees **what** they're being sent before accepting (except for trusted devices where you enabled auto-accept).

### 🚀 Speed and Reliability
- ZSTD on-the-fly compression: documents and code fly multiple times faster.
- Smart chunk sizing matched to the file size.
- Resume interrupted transfers from the point of interruption.
- Folders of any nesting depth - with a single ticket.
- Bandwidth limiting when you don't want to saturate the link.

### 🖥️ Nice Little Touches
Dark / light theme, system tray and autostart ("PC always ready to receive"), transfer history with "send again," two languages (RU / EN), system notifications, CLI version for terminals and scripts.

---

## 5. Why This Is Cool

Because NetDrop brings back a simple idea: **your files are your files.**

- **No server, no problem.** Nothing to hack, nothing to leak, nothing to "go down at 3 AM." As long as two devices are alive and there's some internet connection - the transfer will go through.
- **No account, no tracking.** You're not a login, not an email, not "user #48293." You just sent a file.
- **No middleman, no censorship or limits.** No one decides for you that a file is "too large," "wrong format," or "violates terms of service, section 47.3."
- **One tool instead of a zoo.** AirDrop for your own gear, cloud for big files, messenger for quick ones, flash drive for reliability - NetDrop wraps it all into a single window.

> "Give people tools that require no trust in a middleman, and the middleman becomes unnecessary."
> - the principle behind all decentralization

And yes - it just feels good: watching a 20 GB file fly directly at the speed of your link while the cloud is still spinning up its "preparing to download" spinner.

---

## 6. Ideas for Future Development

- **One-tap file open** - receive on phone > tap > opens in the appropriate app.
- **Download counter** on multi-recipients: how many people have grabbed the file and who's downloading right now.
- **Text notes and clipboard sync** - pass a link or password between devices as easily as a file.
- **Sync folders** between trusted devices: drop a file into a folder on the PC > it appears on the phone.
- **iOS version** - Tauri already supports it; it's just a matter of building it.
- **Group "rooms"**: several family devices in one trusted circle.
- **Auto-clear history and "vanishing" transfers** for the paranoid (we love you).
- **Speed profiles**: "at night - full speed," "during the day - don't disturb calls."
- **File-manager plugins** - "Send via NetDrop" in the right-click context menu.

---

*NetDrop is part of the SmartHoldem / NETFORY ecosystem. A decentralized internet starts with small tools that don't ask for permission.*

> "The best way to predict the future of the internet is to give it back to the people."
