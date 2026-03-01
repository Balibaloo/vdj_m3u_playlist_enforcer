# VirtualDJ M3U Playlist Enforcer

This CLI watches for VirtualDJ `.vdjfolder` playlist descriptors, converts each to a one-to-one `.m3u` playlist, and removes the original descriptor so VirtualDJ can be pointed at the generated files.

## Running locally

```sh
vdj-m3u-playlist-enforcer --config /etc/vdj-m3u-playlist-enforcer/config.toml --once
```

## Packaging & installation

See [docs/INSTALL.md](docs/INSTALL.md) for instructions on building a tarball, deploying the binary/config, and enabling the systemd service and timer that keep the watcher running on boot and kick off hourly scans. The service skips running until `/etc/vdj-m3u-playlist-enforcer/config.toml` exists with a `root_folder` entry.
