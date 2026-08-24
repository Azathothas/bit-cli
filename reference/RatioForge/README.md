# RatioForge

[![Build RatioForge](https://github.com/tsautier/RatioForge/actions/workflows/build.yml/badge.svg)](https://github.com/tsautier/RatioForge/actions/workflows/build.yml)
[![Release RatioForge](https://github.com/tsautier/RatioForge/actions/workflows/release.yml/badge.svg)](https://github.com/tsautier/RatioForge/actions/workflows/release.yml)

**RatioForge** is a modern, .NET 8-powered torrent client simulator that allows you to simulate upload and download statistics with BitTorrent trackers.

> **Note**: This project is a fork and modernization of [RatioMaster.NET](https://github.com/NikolayIT/RatioMaster.NET) by Nikolay Kostov. See [NOTICE.md](NOTICE.md) for full attribution.

## Features

- **Standalone Application**: Does NOT rely on your BitTorrent client (uTorrent, qBittorrent, etc.)
- **No Real Transfer**: Does NOT download/upload actual files - only simulates stats
- **Wide Client Support**: Hardcoded emulations for popular BitTorrent clients:
  - uTorrent (multiple versions)
  - BitComet, Azureus/Vuze, BiglyBT
  - ABC, BitLord, BTuga
  - BitTornado, Burst, BitTyrant, BitSpirit
  - Deluge, Transmission, KTorrent
  - And more!
- **Modernized**: Rebuilt for .NET 8 with improved performance and Windows 11 support

## Requirements

- **Windows 10/11** (64-bit recommended)
- The standard self-contained executable does not require a separate .NET installation.
- The much smaller Lite executable requires the [.NET 8 Desktop Runtime for Windows x64](https://dotnet.microsoft.com/download/dotnet/8.0).

## Installation

1. Download the latest release from the [Releases](https://github.com/tsautier/RatioForge/releases) page.
2. Choose `RatioForge-<version>-win-x64-lite.exe` for the smallest download when .NET 8 Desktop Runtime is installed.
3. Choose `RatioForge-<version>-win-x64.exe` or the zip archive for a self-contained build that needs no separate runtime.
4. Extract the archive when using the zip package, then run the executable.

## Building from Source

### Prerequisites

- [.NET 8 SDK](https://dotnet.microsoft.com/download/dotnet/8.0)
- Visual Studio 2022 (or later) or Rider

### Build Steps

```bash
# Clone the repository
git clone https://github.com/tsautier/RatioForge.git
cd RatioForge

# Restore dependencies
dotnet restore Source/RatioForge.sln

# Build
dotnet build Source/RatioForge.sln --configuration Release

# Run
dotnet run --project Source/RatioForge/RatioForge.csproj
```

### Running Tests

```bash
dotnet test Source/RatioForge.sln
```

## Usage

1. Load a `.torrent` file
2. Select the BitTorrent client to emulate
3. Configure upload/download speeds and ratio
4. Click "Start" to begin sending fake stats to the tracker

For detailed usage instructions, see the built-in help menu.

## What's New in 1.0.13 (RatioForge)

- **Current Client IDs**: Added qBittorrent 5.2.3, Transmission 4.1.3, KTorrent 26.04.3, and BiglyBT 4.1.0.0 profiles
- **Source-Verified Signatures**: Peer ID prefixes and User-Agents are derived from the clients' official tagged source code
- **Compatibility**: Existing legacy profiles remain available alongside the new defaults

See [CHANGELOG.md](CHANGELOG.md) for full version history.
The source evidence for current emulation signatures is recorded in the [2026 client profile audit](docs/client-profile-audit-2026-08-11.md).

## History

**RatioForge** is based on **RatioMaster.NET** by Nikolay Kostov:
- Original Development: 2006-2016
- Original Author: [Nikolay Kostov (NikolayIT)](http://nikolay.it)
- Original Repository: https://github.com/NikolayIT/RatioMaster.NET

This fork was created in 2026 to modernize the project for current .NET standards while preserving all original functionality.

## License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.

### Attribution

- **Original Work** (2006-2016): Copyright © Nikolay Kostov
- **Derivative Work** (2026-present): Copyright © Thomas SAUTIER

See [NOTICE.md](NOTICE.md) for full attribution details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## Disclaimer

This software is for educational purposes only. Use at your own risk. The developers are not responsible for any misuse or damage caused by this program.

## Support

- **Issues**: [GitHub Issues](https://github.com/tsautier/RatioForge/issues)
- **Discussions**: [GitHub Discussions](https://github.com/tsautier/RatioForge/discussions)

---

**Made with ❤️ using .NET 8**  
**Based on RatioMaster.NET by Nikolay Kostov**
