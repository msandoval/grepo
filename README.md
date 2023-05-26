# grepo

Command line helper application for a multi-repo project workflow

Extra tools to help git user keep track of data within a group of repos

```text
OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

SUBCOMMANDS:
    base-dir         Show/set base directory of repos
    branch           Commands for repo branches
    config-path      Show location of config file
    help             Print this message or the help of the given subcommand(s)
    scan-base-dir    Replaces the watched repo list with a list from current base directory
    show-config      Show a list of settings saved
    watch            Commands for watched repos
```

## Quick Start
1. Setup your base directory of repos
```
grepo base-dir <base directory of repos>
```
2. You can now either add the repos from the base directory to watch manually:
```
grepo watch add <repo name>
```
or you can scan the base directory to gather a list of all git repos to watch:
```
grepo scan-base-dir
```

## Using grepo

This tool was created to help git users whose workflow contains multiple repos. In many instances, you will want to see summarized data about the status
of the repo(s) that you are working on. That is where grepo comes in. Grepo watches a list of repos you want and performs actions on those repos such as
get a summarized list of the current branches on each watched repo is on 
```
grepo branch curr
```

or search for a string amongst the branches and return a list of branches that 
match and what repos they are in
```
grepo branch search ma
```

## Current version
### Version 0.1.2
- Using tabled for search output