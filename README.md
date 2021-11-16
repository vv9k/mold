# mold

A simple configuration template rendering program.

The main goal of mold is to allow users to easily switch configuration files between different
contexts. One example usage would be to have custom themes for multiple programs with one easy
way to switch all of their configuration at once.

The context file contains multiple namespaces as well as a global namespace. Each namespace can
have multiple key-value entries. Those variables can then be used in the templates like this:
{% variable1 %}. The name of the variable is enclosed in `{%` and `%}` with any amount of
whitespace in between allowed.

## Context

Here is an example context file that could be used to render configuration:

```yaml
renders:
  alacritty.yml: ~/.config/alacritty/alacritty.yml
  bspwm/bspwmrc: ~/.config/bspwm/bspwmrc
  .gtkrc-2.0: ~/.config/.gtkrc-2.0 # ~ will correctly expand to the home directory
  gtk-3.0/settings.ini: ~/.config/gtk-3.0/settings.ini
namespaces:
  - name: GLOBAL
    variables:
      _font_: JetBrains Mono
      _font_size_: '11'

      alacritty.font: "{%_font_%}"
      alacritty.font-size: "{%_font_size_%}"

      _gtk_theme_: Aritim-Dark
      gtk3.font.name: "{%_font_%}"
      gtk3.font.size: "{%_font_size_%}"
      gtk3.theme.name: "{%_gtk_theme_%}"
      gtk3.icon-theme.name: "Papirus-Dark"
      gtk2.theme.name: "{%_gtk_theme_%}"
      gtk2.font.name: "{%_font_%}"
      gtk2.font.size: "{%_font_size_%}"

      #bspwm.screen.init: randr
      bspwm.screen.init: randr rotate

      _wallpapers_path_: /usr/share/wallpapers

      vim.bg.light_or_dark: dark

  - name: gruvbox
    variables:
      alacritty.theme: gruvbox
      gtk3.theme.name: gruvbox-gtk
      wallpaper.screen0: "{%_wallpapers_path_%}/gruvbox_vertical.png"
      wallpaper.screen1: "{%_wallpapers_path_%}/gruvbox.png"
  - name: solarized
    variables:
      alacritty.theme: solarized
      gtk3.theme.name: Solarized-Dark-Orange
      wallpaper.screen0: "{%_wallpapers_path_%}/solarized_vertical.png"
      wallpaper.screen1: "{%_wallpapers_path_%}/solarized.png"
```

If a variable value is not available in the specified namespace one from `GLOBAL` namespace will be used.

## Installation
To install **mold** you'll need the latest rust with cargo.
```shell
$ cargo build --release && cp ./target/release/mold /usr/bin/
```

## Usage

### Render context directly
If the context contains the `renders` field then it can be rendered directly with:
```shell
$ mold render-context context.yml

$ mold render-context context.yml -n some-namespace
```

### Render specified files
If you want to render files directly use the `render` subcommand:
```shell
$ mold render -c context.yml file1 file2   # will print both files to stdout

$ mold render -c context.yml file1 file2 -n some-namespace

$ mold render -c context.yml file1 file2 -o /tmp  # will save the rendered files as /tmp/file1 and /tmp/file2
```

### Display a diff
``` shell
$ mold diff -c context.yml gtkrc-template ~/.gtkrc-2.0 # will render gtkrc-template and show a diff with ~/.gtkrc-2.0
```

You can checkout the context file that I use for my setup for further examples [here](https://github.com/vv9k/configs/blob/master/mold/context.yml)

## License
[MIT](https://github.com/vv9k/mold/blob/master/LICENSE)
