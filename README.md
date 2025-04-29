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
* `use`:           Select a MCP server to interactive with.
* `list_tools`:    List all tools on the current server.
* `inspect_tool`:  Inspect detailed info of a tool.
* `call_tool`:     Call a tool on the current server.

The `help` command will print out the list of available commands. 

## Copyright
Copyright (c) 2025 Mengxiao Lin. Released under MIT License. Check [LICENSE](./LICENSE) file for more details.