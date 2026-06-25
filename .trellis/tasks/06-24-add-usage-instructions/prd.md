# Add usage instructions for desktop upload and MCP setup

## Goal

Document how to use the desktop upload service, how to build the MCP server image, and how to add the MCP server to Claude.

## Requirements

- Document the desktop upload service entry point and the config file location.
- Document how to build the MCP server Docker image from the repository root.
- Document how to run the MCP server container with the shared image mount.
- Document how to register the MCP server in Claude Desktop / Claude MCP config.
- Keep the instructions aligned with the current repository commands and paths.

## Acceptance Criteria

- [ ] README contains end-to-end usage instructions for the desktop upload service.
- [ ] README contains the Docker build command for `sync-image-mcp`.
- [ ] README contains a Claude MCP configuration example.
- [ ] README uses paths and commands that match the current repository layout.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
