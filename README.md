nah: **N**ot **A** **H**uman
===
A user agent for exploring [Model Context Protocol](https://modelcontextprotocol.io).

**WARNING**: Still in active development, use at your own risk!

## Quick intro
`nah` supports a MCP config file in the [Claude desktop app config](https://modelcontextprotocol.io/quickstart/user) format.
```bash
$ nah ~/mcp/config.json
```

After launching `nah`, it will active all MCP servers declared in the config file and provide a shell-like user interface. Here are some useful commands supported by `nah`.
* `use`:              Select a MCP server to interactive with.
* `list_tools`:       List all tools on the current server.
* `call_tool`:        Call a tool on the current server.
* `list_resources`:   List all resources on the current server.
* `get_resources`:    Read resources through a URI.

The `help` command will print out the list of available commands. 

All commuications (JSON-RPC messages) will be stored in a directory in the current working directory. A `.jsonl` file will be create for each server. Use `--history-path` argument to set the path to store all these records:
```bash
$ nah ~/mcp/config.json --history-path trial_1_history
# History will be stored as trial_1_history/[server name].jsonl
```

## Configuration
By deault, users are asked to provide arguments for tool calls through editing a file in `vi`. Environment variable `$EDITOR` controls the editor to use:
```bash
$ EDITOR=nano nah ~/mcp/config.json
# nano will be used as the editor
```

## Copyright
Copyright (c) 2025 Mengxiao Lin. Released under MIT License. Check [LICENSE](./LICENSE) file for more details.
