# `buckets completions`

Generate shell completion scripts for the `buckets` CLI.

## Usage

```bash
buckets completions <shell>
```

Supported shells:
- `bash`
- `zsh`
- `fish`
- `powershell`
- `elvish`

The script is written to stdout.

## Examples

### Zsh

```bash
mkdir -p ~/.zfunc
buckets completions zsh > ~/.zfunc/_buckets
autoload -U compinit && compinit
```

Ensure `~/.zfunc` is in your `fpath` (e.g. in `~/.zshrc`):

```bash
fpath=(~/.zfunc $fpath)
```

### Bash

```bash
buckets completions bash > /usr/local/etc/bash_completion.d/buckets
```

### Fish

```bash
mkdir -p ~/.config/fish/completions
buckets completions fish > ~/.config/fish/completions/buckets.fish
```

