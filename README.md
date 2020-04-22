# Tundra
Tundra is an open-source MyAnimeList scrobbler application for Linux. It automatically detects media players running on your computer, checks for any anime videos playing, and marks them as watched on you MAL profile.

![Screenshot of Tundra](data/screenshot1.png)

## Download
### Snap Store
[![Get it on Snap Store](https://snapcraft.io/static/images/badges/en/snap-store-black.svg)](https://snapcraft.io/tundra)

### AppImage
If you do not have Snap installed, or do not want to install Tundra as a Snap package, there is also an AppImage version that can be found on [GitHub Releases](https://github.com/m4tx/tundra/releases). Just download the .AppImage file, check the file permissions to allow running it as an executable, and then run it.

## Usage

### GUI
The usage is very simple:

1. First, you need to sign in to your MyAnimeList account with your MAL username and password.
2. Make sure that the title you are about to watch is marked as "watching" or "plan to watch" on your MAL account.
3. Then, you need to run an MPRIS-enabled (see the FAQ section below) media player and play a local anime video file. Its title, poster picture and episode number will appear after a few seconds in Tundra window.
4. The episode will be scrobbled after you watch over half of the video. You will know once you see the notification!

### CLI
Tundra has CLI interface as well. First, you need to authenticate:

```
tundra authenticate <username> <password>
```

Then, you can run Tundra as a daemon:

```
tundra daemon
```

This way, Tundra will periodically check for players running and scrobble any anime videos to your MAL account, just like the GUI version. 

## Building
### Requirements
* [Rust stable](https://www.rust-lang.org/)
* D-Bus
* libnotify
* GTK 3

### How to build
Make sure you have executed `git clone --recursive`, so the submodules are cloned as well. Tundra uses [*Cargo*](https://doc.rust-lang.org/cargo/) as its package manager and build system. It can be built by executing `cargo build` in the project root directory. For the release version, execute `cargo build --release`.

### Snap
After you have installed [*snap*](https://snapcraft.io/) and *snapcraft* execute `snapcraft` in the project root directory to build the Snap package.

## Related projects
* [Taiga](https://github.com/erengy/taiga)
* [Anime Relations](https://github.com/erengy/anime-relations)
* [Anitomy](https://github.com/erengy/anitomy)
* [MyAnimeList Unofficial API Specification](https://github.com/SuperMarcus/myanimelist-api-specification)
