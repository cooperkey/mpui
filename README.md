# mpui

lightweight tui music player. rust. vim-native. no bloat.

## screenshots

![sample 1](./img/img1.png)
![sample 2](./img/img2.png)

## installation

```sh
git clone https://github.com/cooperkey/mpui.git
cd mpui
cargo install --path .
```

## build

```sh
cargo build --release
```

## run

```sh
./target/release/mpui
```

> [!NOTE]
> or if you installed in the path, just run mpui

## configure

```sh
mkdir -p ~/.config/mpui
cp config/default.toml ~/.config/mpui/config.toml
# edit to taste
```

## keybindings

| key     | action        |
| ------- | ------------- |
| `j`/`k` | move down/up  |
| `enter` | play selected |
| `space` | toggle pause  |
| `/`     | search        |
| `:`     | command mode  |
| `q`     | quit          |

see `config.toml` for full keybinding reference.

> [!TIP]
> You can use the dl.sh script to download videos, in different formats, including downloading playlists in yt

> [!NOTE]
> this is vibe coded
