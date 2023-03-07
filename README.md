yaml-configured command launcher for zsh

Install and add to .zshrc:

```bash
ctrl_e_menu() { zle -U "$(read -ek | zlelaunch .ctrl_e.yml)
" }
zle -N ctrl_e_menu
bindkey '^e' ctrl_e_menu
```

Press `<ctrl>+e` to bring up the list of commands, press indicated keys to execute.

```bash
a cargo test --examples --frozen
c cargo clippy --no-deps
z vim .ctrl_e.yml
```

The configuration file should be a list where each entry is either a string or a map
with the keys `command` and optionally `key`

```yaml
- cargo test --examples --frozen
- key: c
  command: cargo clippy --no-deps
 ```
